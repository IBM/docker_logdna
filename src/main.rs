mod consume_log;
mod get_container;
mod html_rec;
mod html_send;
mod logdna_send;
mod structs;

#[tokio::main]
async fn main() {
    const SOCKET_PATH: &str = "/run/docker/plugins/logdna.sock";

    html_rec::await_connections(SOCKET_PATH.to_string()).await;
}
