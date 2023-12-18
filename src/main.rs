use tokio::runtime::Runtime;

mod consume_log;
mod get_container;
mod html_rec;
mod html_send;
mod logdna_send;
mod structs;

use html_rec::await_connections;

fn main() {
    const SOCKET_PATH: &str = "/run/docker/plugins/logdna.sock";

    let rt = Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(await_connections(&rt, SOCKET_PATH.to_string()));
}
