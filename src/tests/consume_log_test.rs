use logdna_client::body::Line;
use ntest::timeout;
use prost::Message;
use std::{ffi::CString, sync::Arc, time::Duration};
use tempfile::TempDir;

use logdna_mock::{mock_server, TestCfg};
use serde_json::from_str;
use tokio::{fs::File, io::AsyncWriteExt, time::sleep};

use crate::{
    consume_log::{cap_str_len, consume_log, decode_protobuf_line},
    get_container::get_client,
    logdna_send::send_lines,
    structs::{CloseNotify, Config, Container, LogEntry, StartRequest},
};

#[tokio::test]
#[timeout(3000)]
// Use the mock server to expect a couple of log lines via TCP.
// The mock server terminates when all lines have been read.
//
// Create a (green) thread mocking Docker. It sends protobuf-encoded log lines over a fifo file
// towards the consume_log call.
//
// Run the consume_log function in a separate (green) thread to receive the protobuf log lines and
// forward them towards the mock server.
async fn consume_log_test() {
    let cfg = TestCfg::new_vec(
        "127.0.0.1:9004".parse().unwrap(),
        vec![
            "You are at the beginning so there must be an end".to_string(),
            "It's simple: Overspecialize and you breed in weakness.".to_string(),
            "The net is vast and infinite.".to_string(),
        ],
    );

    // create fifo file
    let temp_dir = TempDir::new().unwrap();
    let fifo_path = temp_dir.path().join("container_log.fifo");
    let fifo_path = fifo_path.to_str().unwrap();
    let fifo_path_c_str = CString::new(fifo_path).unwrap();

    // simulate docker creating the fifo file
    unsafe {
        libc::mkfifo(fifo_path_c_str.as_ptr(), 0o644);
    }

    let close_notify = Arc::new(CloseNotify::new());
    let start_request: StartRequest =
        from_str((r#"{"File": ""#.to_string() + fifo_path + r#"","Info": {"ContainerID": "","ContainerLabels": {"test_label": "test_value", "a": "b"}}}"#).as_str()).unwrap();
    let config = Config {
        hostname: "test-i-dont-care".to_string(),
        logdna_host: "127.0.0.1:9004".to_string(),
        api_key: "my-cool-api-key".to_string(),
        ip: Some("111.111.111.111".to_string()),
        mac: Some("0014a541616b".to_string()),
        tags: "something,some,asfd".to_string(),
        app: "app".to_string(),
        level: None,
        // max_length does get enforced in consume_message function
        max_length: 100,
        for_mock_server: true,
        flush_interval: Duration::from_millis(250),
        max_buffer_size: 2097152,
        http_client_timeout: Duration::from_millis(30000),
        max_request_retry: 5,
    };
    let client = get_client(&config).await.unwrap();
    let container = Container {
        start_request,
        config,
        client,
    };
    println!("{}", container.start_request.file);

    tokio::join!(
        consume_log_test_docker(close_notify.clone(), fifo_path),
        consume_log(close_notify.clone(), container),
        mock_server(cfg)
    );
}
// this function simulates the behaviour of the start_logging and stop_logging requests in html_rec
// therefore it simulates everything docker sends and does to the plugin
async fn consume_log_test_docker(close_notify: Arc<CloseNotify>, fifo_path: &str) {
    let mut file = File::create(&fifo_path).await.unwrap();

    // wait for server to boot up
    sleep(Duration::from_millis(100)).await;

    let log_entry1 = LogEntry {
        source: "stdout".to_string(),
        time_nano: 12341234143,
        line: "You are at the beginning so there must be an end"
            .as_bytes()
            .to_vec(),
        partial: false,
        partial_log_metadata: None,
    };
    let mut log_entry2 = log_entry1.clone();
    log_entry2.line = "It's simple: Overspecialize and you breed in weakness."
        .as_bytes()
        .to_vec();
    let mut log_entry3 = log_entry1.clone();
    log_entry3.line = "The net is vast and infinite.".as_bytes().to_vec();
    // simulate docker writing stuff to the pipe
    {
        let encoded_len = log_entry1.encoded_len();
        let len_buf = (encoded_len as u32).to_be_bytes();
        file.write_all(&len_buf).await.unwrap();
        let mut buf: Vec<u8> = Vec::new();
        buf.reserve(encoded_len);
        log_entry1.encode(&mut buf).unwrap();
        file.write_all(&buf).await.unwrap();
    }
    {
        let encoded_len = log_entry2.encoded_len();
        let len_buf = (encoded_len as u32).to_be_bytes();
        file.write_all(&len_buf).await.unwrap();
        let mut buf: Vec<u8> = Vec::new();
        buf.reserve(encoded_len);
        log_entry2.encode(&mut buf).unwrap();
        file.write_all(&buf).await.unwrap();
    }
    // simulate docker sending a request to /LogDriver.StopLogging
    close_notify.requested_to_close.notify_one();
    // simulate the last message being stuck in a very slow pipe
    sleep(Duration::from_millis(400)).await;
    {
        let encoded_len = log_entry3.encoded_len();
        let len_buf = (encoded_len as u32).to_be_bytes();
        file.write_all(&len_buf).await.unwrap();
        let mut buf: Vec<u8> = Vec::new();
        buf.reserve(encoded_len);
        log_entry3.encode(&mut buf).unwrap();
        file.write_all(&buf).await.unwrap();
    }
    // wait to be allowed to tell docker to close the pipe
    close_notify.let_docker_close_pipe.notified().await;
    // simulate docker closing the pipe
    drop(file);
}

#[tokio::test]
#[timeout(3000)]
// Use the mock server to expect a couple of log lines via TCP.
// The mock server terminates when all lines have been read.
//
// Run the consume_message function in a separate (green) thread to send the log lines towards the
// mock server.
async fn consume_message_test() {
    let cfg = TestCfg::new_vec(
        "127.0.0.1:9002".parse().unwrap(),
        vec![
            "Nothing ends but everything must rest.".to_string(),
            "It was our one hundred and ninth year in the computer.".to_string(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        ],
    );
    tokio::join!(
        consume_message_test_sender(),
        // check labels and such can't be checkd because of private fields in logdna server mocking library
        mock_server(cfg),
    );
}
async fn consume_message_test_sender() {
    let start_request: StartRequest =
        from_str(r#"{"File": "","Info": {"ContainerID": "","ContainerLabels": {"test_label": "test_value", "a": "b"}}}"#).unwrap();
    let config = Config {
        hostname: "test-i-dont-care".to_string(),
        logdna_host: "127.0.0.1:9002".to_string(),
        api_key: "my-cool-api-key".to_string(),
        ip: Some("111.111.111.111".to_string()),
        mac: Some("0014a541616b".to_string()),
        tags: "something,some,asfd".to_string(),
        app: "app".to_string(),
        level: None,
        // max_length does get enforced in consume_message function
        max_length: 100,
        for_mock_server: true,
        flush_interval: Duration::from_millis(250),
        max_buffer_size: 2097152,
        http_client_timeout: Duration::from_millis(30000),
        max_request_retry: 5,
    };
    let client = get_client(&config).await.unwrap();
    let container = Container {
        start_request,
        config,
        client,
    };

    // wait for server to boot up
    sleep(Duration::from_millis(100)).await;

    let mut lines: Vec<Line> = Vec::new();
    {
        let log_entry = LogEntry {
            source: "stdout".to_string(),
            time_nano: 12341234143,
            line: "Nothing ends but everything must rest.".as_bytes().to_vec(),
            partial: false,
            partial_log_metadata: None,
        };
        let encoded_len = log_entry.encoded_len();
        let mut buf: Vec<u8> = Vec::new();
        buf.reserve(encoded_len);
        log_entry.encode(&mut buf).unwrap();
        lines.push(decode_protobuf_line(&container, &buf).await.unwrap());
    }
    {
        let log_entry = LogEntry {
            source: "stdout".to_string(),
            time_nano: 12341234143,
            line: "It was our one hundred and ninth year in the computer."
                .as_bytes()
                .to_vec(),
            partial: false,
            partial_log_metadata: None,
        };
        let encoded_len = log_entry.encoded_len();
        let mut buf: Vec<u8> = Vec::new();
        buf.reserve(encoded_len);
        log_entry.encode(&mut buf).unwrap();
        lines.push(decode_protobuf_line(&container, &buf).await.unwrap());
    }
    {
        let log_entry = LogEntry {
            source: "stdout".to_string(),
            time_nano: 12341234143,
            line: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .as_bytes()
                .to_vec(),
            partial: false,
            partial_log_metadata: None,
        };
        let encoded_len = log_entry.encoded_len();
        let mut buf: Vec<u8> = Vec::new();
        buf.reserve(encoded_len);
        log_entry.encode(&mut buf).unwrap();
        lines.push(decode_protobuf_line(&container, &buf).await.unwrap());
    }
    send_lines(&container, lines, false).await.unwrap();
}

#[tokio::test]
#[timeout(3000)]
#[should_panic(expected = "timeout: the function call took 3000 ms. Max time 3000 ms")]
// expect one line, don't send any
async fn consume_message_should_hang_test() {
    let cfg = TestCfg::new_vec("127.0.0.1:9005".parse().unwrap(), vec!["test".to_string()]);
    tokio::join!(
        consume_message_should_hang_test_sender(),
        // check labels and such can't be checkd because of private fields in logdna server mocking library
        mock_server(cfg),
    );
}
async fn consume_message_should_hang_test_sender() {
    // wait for server to boot up
    sleep(Duration::from_millis(100)).await;
    // don't send anything
}

#[tokio::test]
async fn cap_str_len_test() {
    assert_eq!(
        cap_str_len(
            "looking like the chimpanzee AM had intended him to resemble.",
            100
        )
        .await,
        "looking like the chimpanzee AM had intended him to resemble."
    );
    assert_eq!(
        cap_str_len(
            "looking like the chimpanzee AM had intended him to resemble.",
            0
        )
        .await,
        "looking like the chimpanzee AM had intended him to resemble."
    );
    assert_eq!(
        cap_str_len(
            "looking like the chimpanzee AM had intended him to resemble.",
            10
        )
        .await,
        "looking li"
    );
    // each emojis consumes 4 bytes
    assert_eq!(cap_str_len("😀😃😄", 0).await, "😀😃😄");
    assert_eq!(cap_str_len("😀😃😄", 1).await, "");
    assert_eq!(cap_str_len("😀😃😄", 2).await, "");
    assert_eq!(cap_str_len("😀😃😄", 3).await, "");
    assert_eq!(cap_str_len("😀😃😄", 4).await, "😀");
    assert_eq!(cap_str_len("😀😃😄", 5).await, "😀");
    assert_eq!(cap_str_len("😀😃😄", 6).await, "😀");
    assert_eq!(cap_str_len("😀😃😄", 7).await, "😀");
    assert_eq!(cap_str_len("😀😃😄", 8).await, "😀😃");
    assert_eq!(cap_str_len("😀😃😄", 9).await, "😀😃");
    assert_eq!(cap_str_len("😀😃😄", 10).await, "😀😃");
    assert_eq!(cap_str_len("😀😃😄", 11).await, "😀😃");
    assert_eq!(cap_str_len("😀😃😄", 12).await, "😀😃😄");
    assert_eq!(cap_str_len("😀😃😄", 13).await, "😀😃😄");
}
