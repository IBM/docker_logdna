use futures::future::join_all;
use logdna_mock::{make_panic_exit, mock_server, TestCfg, TestType};
use std::{net::SocketAddr, time::Duration};
use tokio::process::Command;

const CONTAINER_AMOUNT: usize = 10;
const LINES_PER_SEC: f64 = 500.0;
const LINES_AMOUNT: usize = 10000;
const MAX_SLOW_STREAK: u64 = 0;
const CHAR_MIN: u8 = ' ' as u8;
const CHAR_MAX: u8 = '~' as u8 + 1;
const STR_LEN_MIN: usize = 40;
const STR_LEN_MAX: usize = 100;
const INGESTER_DELAY: u64 = 0;
const SERVER_TIMEOUT: u64 = 120000;

#[tokio::main]
async fn main() {
    make_panic_exit();

    let futures = (0..CONTAINER_AMOUNT).map(|i| start_client_server_pair(i));
    join_all(futures).await;
}

async fn start_client_server_pair(i: usize) {
    let port = 9000 + i as u16;
    let server_addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();
    let seed = 69420 + i as u64;

    let cfg = TestCfg {
        addr: server_addr,
        seed,
        lines: Vec::new(),
        test_type: TestType::Rng,
        lines_per_sec: LINES_PER_SEC,
        lines_amount: LINES_AMOUNT,
        max_slow_streak: MAX_SLOW_STREAK,
        char_range: CHAR_MIN..CHAR_MAX,
        string_len_range: STR_LEN_MIN..STR_LEN_MAX,
        ingester_delay: Duration::from_millis(INGESTER_DELAY),
        server_timeout: Duration::from_millis(SERVER_TIMEOUT),
    };

    println!("starting client/server pair {i}");

    // are we running in docker in docker or in github action
    let in_dind = std::env::var("IN_DIND").unwrap().parse::<bool>().unwrap();

    let return_status = tokio::join!(
        mock_server(cfg),
        Command::new(match in_dind {
            true => "docker-entrypoint.sh",
            false => "docker",
        })
        .arg("run")
        // .arg("--net")
        // .arg("host")
        // .arg("debian")
        // .arg("bash")
        // .arg("-c")
        // .arg(format!("apt-get update && apt-get install -y net-tools nmap && ifconfig && nmap -p {port} 127.0.0.1/0"))
        .arg("--log-driver")
        .arg("logdna")
        .arg("--log-opt")
        .arg("api_key=i-dont-care")
        .arg("--log-opt")
        .arg(format!(
            "logdna_host={}:{}",
            match in_dind {
                true => "test-client",
                false => "::1",
            },
            port
        ))
        .arg("--log-opt")
        .arg("for_mock_server=true")
        .arg("-t")
        .arg("-e")
        // doesn't actually matter at all
        .arg(format!("MOCK_ADDRESS=127.0.0.1:{port}"))
        .arg("-e")
        .arg(format!("MOCK_SEED={seed}"))
        .arg("-e")
        .arg(format!("MOCK_LINES_PER_SEC={LINES_PER_SEC}"))
        .arg("-e")
        .arg(format!("MOCK_LINES_AMOUNT={LINES_AMOUNT}"))
        .arg("-e")
        .arg(format!("MOCK_MAX_SLOW_STREAK={MAX_SLOW_STREAK}"))
        .arg("-e")
        .arg(format!("MOCK_CHAR_MIN={CHAR_MIN}"))
        .arg("-e")
        .arg(format!("MOCK_CHAR_MAX={CHAR_MAX}"))
        .arg("-e")
        .arg(format!("MOCK_STR_LEN_MIN={STR_LEN_MIN}"))
        .arg("-e")
        .arg(format!("MOCK_STR_LEN_MAX={STR_LEN_MAX}"))
        .arg("-e")
        .arg(format!("MOCK_INGESTER_DELAY={INGESTER_DELAY}"))
        .arg("-e")
        .arg(format!("MOCK_SERVER_TIMEOUT={SERVER_TIMEOUT}"))
        .arg("mock_client:latest")
        .spawn()
        .unwrap()
        .wait_with_output()
    )
    .1
    .unwrap()
    .status;

    assert_eq!(return_status.code(), Some(0));
}

// async fn test() {
//     Command::new("journalctl")
//         .arg("--user")
//         .arg("-fu")
//         .arg("docker.service")
//         .arg("--no-pager")
//         .spawn()
//         .unwrap()
//         .wait_with_output()
//         .await
//         .unwrap();
// }
