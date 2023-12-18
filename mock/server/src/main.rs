use tokio::time::Duration;

use logdna_mock::{make_panic_exit, mock_server, TestCfg, TestType};

#[tokio::main]
async fn main() {
    make_panic_exit();

    let cfg = TestCfg {
        addr: "127.0.0.1:9000".parse().unwrap(),
        seed: 69420,
        lines: Vec::new(),
        test_type: TestType::LineLength,
        lines_per_sec: 20.0,
        lines_amount: 1000000,
        max_slow_streak: 0,
        char_range: (' ' as u8)..('~' as u8 + 1),
        // string_len_range: 99..100,
        string_len_range: 99..100,
        ingester_delay: Duration::from_millis(0),
        server_timeout: Duration::from_millis(600000),
    };
    // let cfg = load_cfg();

    println!("starting the server");
    mock_server(cfg).await;
}
