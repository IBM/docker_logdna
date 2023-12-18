use logdna_client::body::Line;
use prost::Message;
use std::str::from_utf8;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::time::{sleep, Duration, Instant};

use crate::logdna_send::{create_logdna_line, send_critical_error, send_lines};
use crate::structs::{CloseNotify, Container, LogEntry};

#[cfg(test)]
#[path = "./tests/consume_log_test.rs"]
mod consume_log_test;

// no idea why clippy is so mad about this
// this clearly is not an issue
#[allow(clippy::read_zero_byte_vec)]
pub async fn consume_log(close_notify: Arc<CloseNotify>, container: Container) {
    let container = Arc::new(container);
    let mut file = match File::open(&container.start_request.file).await {
        Ok(v) => v,
        Err(e) => {
            send_critical_error(
                &container,
                format!("Failed to open docker fifo file: {}", e),
            )
            .await;
            close_notify.let_docker_close_pipe.notify_one();
            return;
        }
    };
    // write message after message without delimiter into this buffer
    let mut buf = Vec::with_capacity(container.config.max_buffer_size);
    // indices of first byte of each message in buf
    let mut start_positions: Vec<usize> = vec![0];
    let mut last_flush = Instant::now();

    loop {
        ////// read message length delimiter //////
        let mut del_buf: [u8; 4] = [0; 4];
        let read_delimiter_future = file.read_exact(&mut del_buf);
        tokio::pin!(read_delimiter_future);

        let delimiter_result;
        tokio::select! {
            ret = &mut read_delimiter_future => {
                delimiter_result = ret;
            }
            _ = close_notify.requested_to_close.notified() => {
                println!("requested to close via http");
                // there might still be partial messages to come
                tokio::select! {
                    ret = read_delimiter_future => {
                        // read this message and then see if there's still more to come
                        // notify_one to not forget that docker requested to close the pipe
                        close_notify.requested_to_close.notify_one();
                        delimiter_result = ret;
                    }
                    // give docker some time to send last message
                    // the problem is that there is no way to figure out if the last message has
                    // already been sent so all we can do is wait
                    _ = sleep(Duration::from_millis(100)) => {
                        // now all messages should have been sent to the pipe by docker -> it's
                        // safe to let docker close the pipe
                        // just to make sure no messages get lost, wait for an EOF on the pipe ->
                        // don't immediately return
                        close_notify.let_docker_close_pipe.notify_one();
                        // we don't need to remember that docker requested to close the pipe
                        continue;
        }   }   }   }
        match delimiter_result {
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                println!("found EOF, closing pipe");
                break;
            }
            Err(e) => {
                send_critical_error(&container, format!("Failed to read delimiter size: {}", e))
                    .await;
                break;
            }
        };
        let msg_len = u32::from_be_bytes(del_buf) as usize;

        ////// read message //////
        let start = buf.len();
        let end = buf.len() + msg_len;
        buf.resize(end, 0);
        if let Err(e) = file.read_exact(&mut buf[start..end]).await {
            send_critical_error(&container, format!("Failed to read message: {}", e)).await;
            break;
        }
        start_positions.push(end);

        ////// should flush? //////
        // this logic is best tested using end to end stress tests
        if buf.len() >= container.config.max_buffer_size
            || (!buf.is_empty()
                && Instant::now().duration_since(last_flush) >= container.config.flush_interval)
        {
            // do this on a separate (green) thread
            // docker needs messages to be read as quickly as possible from the pipe
            // any delay can kill the pipe when sending more than 1000 lines a second
            // that would expose a DOS attack vector
            tokio::spawn(consume_buf(container.clone(), buf, start_positions));
            last_flush = Instant::now();
            // this is the only really slow and blocking operation in the loop
            buf = Vec::with_capacity(container.config.max_buffer_size);
            start_positions = vec![0];
        }
    }
    if !buf.is_empty() {
        tokio::spawn(consume_buf(container, buf, start_positions));
    }
    // signal request to /LogDriver.StopLogging that all messages have been consumed
    println!("finished consuming log");
    close_notify.let_docker_close_pipe.notify_one();
}

async fn consume_buf(container: Arc<Container>, buf: Vec<u8>, start_positions: Vec<usize>) {
    let mut lines: Vec<Line> = Vec::with_capacity(start_positions.len() - 1);
    for (&start, &end) in start_positions.iter().zip(start_positions.iter().skip(1)) {
        match decode_protobuf_line(&container, &buf[start..end]).await {
            Ok(l) => lines.push(l),
            Err(e) => send_critical_error(&container, e).await,
        };
    }
    // ignore result
    // when sending fails after retries there's nothing possible to be done
    _ = send_lines(&container, lines, false).await;
}

async fn decode_protobuf_line(container: &Container, buf: &[u8]) -> Result<Line, String> {
    let log_entry: LogEntry = match Message::decode(buf) {
        Ok(v) => v,
        Err(e) => return Err(format!("Failed to decode log entry: {}", e)),
    };

    let line = match from_utf8(&log_entry.line) {
        Ok(v) => v,
        Err(e) => return Err(format!("Failed to utf8 decode log line: {}", e)),
    };
    let line = cap_str_len(line, container.config.max_length)
        .await
        .to_string();
    create_logdna_line(container, line).await
}

// len 0 -> keep string as is
async fn cap_str_len(mut string: &str, len: usize) -> &str {
    let mut len = match len {
        0 => return string,
        v => v,
    };
    // cut before len and not through utf8 character
    loop {
        string = match string.get(0..len) {
            Some(v) => v,
            None => {
                len -= 1;
                continue;
            }
        };
        return string;
    }
}
