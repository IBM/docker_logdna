use serde_json::json;
use std::str::from_utf8;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::AsyncReadExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::time::sleep;

use crate::html_send::{return_err, return_ok};

#[tokio::test]
async fn return_err_test() {
    let temp_dir = TempDir::new().unwrap();
    let sock_path = temp_dir.path().join("logdna.sock");
    let sock_path = sock_path.to_str().unwrap().to_string();

    tokio::join!(
        return_err_docker(sock_path.clone()),
        return_err_plugin(sock_path)
    );
}

async fn return_err_docker(sock_path: String) {
    let listener = UnixListener::bind(sock_path).unwrap();
    {
        let mut stream = listener.accept().await.unwrap().0;
        let mut buf: [u8; 300] = [0; 300];
        stream.read(&mut buf).await.unwrap();
        let text = from_utf8(&buf).unwrap();
        let expected_text = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 68\r\nContent-Type: application/json\r\n\r\n{\"Err\":\"Critical Error: something went badly wrong, please help!\"}\r\n\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(text, expected_text);
    }
    {
        let mut stream = listener.accept().await.unwrap().0;
        let mut buf: [u8; 300] = [0; 300];
        stream.read(&mut buf).await.unwrap();
        let text = from_utf8(&buf).unwrap();
        let expected_text = "HTTP/1.1 404 Internal Server Error\r\nContent-Length: 54\r\nContent-Type: application/json\r\n\r\n{\"Err\":\"The site you're requesting isn't available\"}\r\n\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(text, expected_text);
    }
    {
        let mut stream = listener.accept().await.unwrap().0;
        let mut buf: [u8; 300] = [0; 300];
        stream.read(&mut buf).await.unwrap();
        let text = from_utf8(&buf).unwrap();
        let expected_text = "HTTP/1.1 451 Internal Server Error\r\nContent-Length: 26\r\nContent-Type: application/json\r\n\r\n{\"Err\":\"YOU'RE WINNER!\"}\r\n\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(text, expected_text);
    }
}

async fn return_err_plugin(sock_path: String) {
    sleep(Duration::from_millis(100)).await;
    {
        let stream = UnixStream::connect(sock_path.clone()).await.unwrap();
        return_err(
            stream,
            "Critical Error: something went badly wrong, please help!".to_string(),
            500,
        )
        .await;
    }
    {
        let stream = UnixStream::connect(sock_path.clone()).await.unwrap();
        return_err(
            stream,
            "The site you're requesting isn't available".to_string(),
            404,
        )
        .await;
    }
    {
        let stream = UnixStream::connect(sock_path.clone()).await.unwrap();
        return_err(stream, "YOU'RE WINNER!".to_string(), 451).await;
    }
}

#[tokio::test]
#[should_panic(expected = "assertion failed: !err.is_empty()")]
// listener can't get dropped before end of scope
#[allow(unused_variables)]
async fn return_err_empty() {
    let temp_dir = TempDir::new().unwrap();
    let sock_path = temp_dir.path().join("logdna.sock");
    let sock_path = sock_path.to_str().unwrap().to_string();

    sleep(Duration::from_millis(100)).await;
    let listener = UnixListener::bind(sock_path.clone()).unwrap();

    let stream = UnixStream::connect(sock_path).await.unwrap();
    return_err(stream, "".to_string(), 500).await;
}

#[tokio::test]
async fn return_ok_test() {
    let temp_dir = TempDir::new().unwrap();
    let sock_path = temp_dir.path().join("logdna.sock");
    let sock_path = sock_path.to_str().unwrap().to_string();

    tokio::join!(
        return_ok_docker(sock_path.clone()),
        return_ok_plugin(sock_path)
    );
}

async fn return_ok_docker(sock_path: String) {
    let listener = UnixListener::bind(sock_path).unwrap();
    {
        let mut stream = listener.accept().await.unwrap().0;
        let mut buf: [u8; 300] = [0; 300];
        stream.read(&mut buf).await.unwrap();
        let text = from_utf8(&buf).unwrap();
        let expected_text = "HTTP/1.1 200 OK\r\nContent-Length: 19\r\nContent-Type: application/json\r\n\r\n{\"test_obj\":\"hi\"}\r\n\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(text, expected_text);
    }
    {
        let mut stream = listener.accept().await.unwrap().0;
        let mut buf: [u8; 300] = [0; 300];
        stream.read(&mut buf).await.unwrap();
        let text = from_utf8(&buf).unwrap();
        let expected_text = "HTTP/1.1 200 OK\r\nContent-Length: 37\r\nContent-Type: application/json\r\n\r\n{\"a\":true,\"b\":64,\"c\":[\"hi\",\"hoho\"]}\r\n\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(text, expected_text);
    }
}

async fn return_ok_plugin(sock_path: String) {
    sleep(Duration::from_millis(100)).await;
    {
        let stream = UnixStream::connect(sock_path.clone()).await.unwrap();
        return_ok(stream, json!({"test_obj": "hi"})).await;
    }
    {
        let stream = UnixStream::connect(sock_path.clone()).await.unwrap();
        return_ok(stream, json!({"a": true, "b": 64, "c": ["hi", "hoho"]})).await;
    }
}
