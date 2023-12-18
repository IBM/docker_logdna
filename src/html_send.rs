use serde_json::{json, Value};
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

#[cfg(test)]
#[path = "./tests/html_send_test.rs"]
mod html_send_test;

pub async fn return_ok<D: AsyncWrite + Unpin>(mut stream: D, content: Value) {
    let content_str = content.to_string() + "\r\n";
    let length = content_str.len();
    let status_line = "HTTP/1.1 200 OK";
    let response = format!("{status_line}\r\nContent-Length: {length}\r\nContent-Type: application/json\r\n\r\n{content_str}");

    match stream.write_all(response.as_bytes()).await {
        Ok(_) => (),
        Err(e) => println!("Error responding to docker: {}", e),
    }
}

pub async fn return_err<D: AsyncWrite + Unpin>(mut stream: D, err: String, err_code: u16) {
    println!("{}", err);
    // empty err message would signal success to docker
    assert!(!err.is_empty());
    let content_str = json!({"Err": err}).to_string() + "\r\n";
    let length = content_str.len();
    let response = format!("HTTP/1.1 {err_code} Internal Server Error\r\nContent-Length: {length}\r\nContent-Type: application/json\r\n\r\n{content_str}");

    match stream.write_all(response.as_bytes()).await {
        Ok(_) => (),
        Err(e) => println!("Error responding to docker: {}", e),
    }
}
