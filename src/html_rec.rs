use serde_json::{from_slice, json};
use std::collections::HashMap;
use std::str::from_utf8;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::UnixListener;
use tokio::runtime::Runtime;

use crate::consume_log::consume_log;
use crate::get_container::get_container;
use crate::html_send::{return_err, return_ok};
use crate::logdna_send::{create_logdna_line, send_lines};
use crate::structs::{CloseNotify, StartRequest, StopRequest};

// These function are very difficult to test.
// MY attempts at mocking docker failed with stack overflow errors.
// The missing tests are not that important since these functions are already being extensively
// tested using end to end tests. Additionally they are very linear in nature.
pub async fn await_connections(rt: &Runtime, socket_path: String) {
    let mut close_notifcations: HashMap<String, Arc<CloseNotify>> = HashMap::new();

    println!("listing on unix socket '{}'", socket_path);
    let listener = match UnixListener::bind(socket_path) {
        Ok(v) => v,
        Err(e) => panic!("Failed to bind socket: {}", e),
    };

    loop {
        let stream = match listener.accept().await {
            Ok(v) => v.0,
            Err(e) => {
                println!("failed to accept incoming connection {}", e);
                continue;
            }
        };
        handle_connection(rt, &mut close_notifcations, stream).await;
    }
}

pub async fn handle_connection<D: AsyncWrite + AsyncRead + Unpin>(
    rt: &Runtime,
    close_notifcations: &mut HashMap<String, Arc<CloseNotify>>,
    mut stream: D,
) {
    // don't accept anything bigger than 1mb
    const BUF_SIZE: usize = 1048576;
    let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
    match stream.read(&mut buf).await {
        Ok(_) => (),
        Err(e) => {
            println!("Critical error: failed to read html message: {}", e);
            return;
        }
    };
    // is this only a ping?
    if buf.iter().all(|&a| a == 0) {
        println!("received ping");
        return;
    }

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    let res = match req.parse(&buf) {
        Ok(v) => v,
        Err(e) => {
            return_err(
                stream,
                format!("Failed to httparse the request: {}", e),
                500,
            )
            .await;
            return;
        }
    };
    let offset = match res {
        httparse::Status::Complete(v) => v,
        httparse::Status::Partial => {
            format!("Request is bigger than {} bytes", BUF_SIZE);
            return;
        }
    };

    let end: usize = match buf.iter().position(|&a| a == 0) {
        Some(v) => v,
        None => BUF_SIZE,
    };
    assert!(end <= BUF_SIZE);
    match req.path {
        Some("/LogDriver.StartLogging") => {
            start_logging(rt, close_notifcations, stream, &buf[offset..end]).await
        }
        Some("/LogDriver.StopLogging") => {
            stop_logging(close_notifcations, stream, &buf[offset..end]).await
        }
        Some("/LogDriver.Capabilities") => return_ok(stream, json!({"ReadLogs":false})).await,
        Some("/LogDriver.ReadLogs") => {
            return_err(stream, "Reading logs is not implemented".to_string(), 500).await
        }
        Some(v) => return_err(stream, format!("Unknown path: {}", v), 404).await,
        None => return_err(stream, format!("No path provided"), 500).await,
    }
}

async fn start_logging<D: AsyncWrite + Unpin>(
    rt: &Runtime,
    close_notifcations: &mut HashMap<String, Arc<CloseNotify>>,
    stream: D,
    jsonbody: &[u8],
) {
    let start_request: StartRequest = match from_slice(jsonbody) {
        Ok(v) => v,
        Err(e) => {
            return_err(
                stream,
                format!(
                    "Failed to parse json: {}\njson str: {}",
                    e,
                    match from_utf8(jsonbody) {
                        Ok(s) => s.to_string(),
                        Err(t) => format!("Couldn't utf8-convert: {}", t),
                    }
                ),
                500,
            )
            .await;
            return;
        }
    };
    match start_request.info.container_name {
        Some(ref v) => println!("Start logging from {} container {}", start_request.file, v),
        None => println!("Start logging unnamed container at {}", start_request.file),
    }
    let container = match get_container(start_request).await {
        Ok(v) => v,
        Err(e) => {
            return_err(stream, format!("{}", e), 500).await;
            return;
        }
    };

    if close_notifcations.contains_key(&container.start_request.file) {
        return_err(
            stream,
            format!(
                "File {} has already been opened",
                container.start_request.file
            ),
            500,
        )
        .await;
        return;
    }
    let close_notify = Arc::new(CloseNotify::new());
    close_notifcations.insert(container.start_request.file.clone(), close_notify.clone());

    // verify logdna server can be talked to
    let line = match create_logdna_line(
        &container,
        "Critical: docker_logdna starting to log".to_string(),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return_err(
                stream,
                format!("Connection to Logdna Host failed with supplied API Key: {e}"),
                500,
            )
            .await;
            return;
        }
    };
    match send_lines(&container, vec![line], true).await {
        Ok(_) => (),
        Err(_) => {
            return_err(
                stream,
                "Connection to Logdna Host failed with supplied API Key".to_string(),
                500,
            )
            .await;
            return;
        }
    }

    println!(
        "watching {} container roughly right now",
        close_notifcations.len()
    );
    rt.spawn(consume_log(close_notify, container));

    return_ok(stream, json!({})).await;
}

async fn stop_logging<D: AsyncWrite + Unpin>(
    close_notifcations: &mut HashMap<String, Arc<CloseNotify>>,
    stream: D,
    jsonbody: &[u8],
) {
    let stop_request: StopRequest = match from_slice(jsonbody) {
        Ok(v) => v,
        Err(e) => {
            return_err(
                stream,
                format!(
                    "Failed to parse json: {}\njson str: {}",
                    e,
                    match from_utf8(jsonbody) {
                        Ok(s) => s.to_string(),
                        Err(t) => format!("Couldn't utf8-convert: {}", t),
                    }
                ),
                500,
            )
            .await;
            return;
        }
    };
    println!("Stop logging at {}", stop_request.file);
    let close_notify = match close_notifcations.remove(&stop_request.file) {
        Some(v) => v,
        None => {
            return_err(
                stream,
                format!("File {} has already been stopped", stop_request.file),
                500,
            )
            .await;
            return;
        }
    };
    close_notify.requested_to_close.notify_one();
    // wait to be allowed to tell docker to close the pipe
    close_notify.let_docker_close_pipe.notified().await;
    println!(
        "watching {} container roughly right now",
        close_notifcations.len()
    );

    return_ok(stream, json!({})).await;
}
