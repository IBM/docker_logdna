use logdna_mock::{load_cfg, make_panic_exit, mock_client};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    make_panic_exit();

    sleep(Duration::from_millis(1000)).await;
    let cfg = load_cfg();
    mock_client(cfg).await;
}
