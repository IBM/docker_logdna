use hyper::Body;
use hyper::Response;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::future::Future;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::ops::Range;
use std::string::String;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::{panic, process};
use tokio::signal;
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

use logdna_mock_ingester::{http_ingester_with_processors, IngestError, Line};

#[derive(Debug, Clone)]
pub enum TestType {
    Vec,
    Rng,
    LineLength,
    // custom check: fail and check the retries work
    Retry,
}

#[derive(Debug, Clone)]
pub struct TestCfg {
    pub addr: SocketAddr,
    pub seed: u64,
    // not available in env config
    pub lines: Vec<String>,
    pub test_type: TestType,

    // set to 0 for as fast as possible
    pub lines_per_sec: f64,
    pub lines_amount: usize,
    // set to 0 to ignore slow streaks
    pub max_slow_streak: u64,

    pub char_range: Range<u8>,
    pub string_len_range: Range<usize>,

    pub ingester_delay: Duration,
    pub server_timeout: Duration,
}

impl TestCfg {
    pub fn new_vec(addr: SocketAddr, lines: Vec<String>) -> TestCfg {
        TestCfg {
            addr,
            seed: 0,
            lines,
            test_type: TestType::Vec,
            lines_per_sec: 1.0,
            lines_amount: 0,
            max_slow_streak: 0,
            char_range: 0..1,
            string_len_range: 0..1,
            ingester_delay: Duration::from_millis(0),
            server_timeout: Duration::from_millis(5000),
        }
    }
}

pub fn rng_check_line(cfg: &TestCfg, rng: &mut ChaCha8Rng, line: &Line) {
    assert_eq!(
        match &line.line {
            Some(v) => Some(v.replace("\r", "")),
            None => None,
        },
        Some(get_str(cfg, rng))
    );
}

pub fn get_str(cfg: &TestCfg, rng: &mut ChaCha8Rng) -> String {
    let len = rng.gen_range(cfg.string_len_range.clone());
    let mut string = String::with_capacity(len);
    for _ in 0..len {
        string.push(rng.gen_range(cfg.char_range.clone()) as char);
    }
    string
}

pub fn get_rng(cfg: &TestCfg) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(cfg.seed)
}

pub fn rng_check_lines(
    cfg: &TestCfg,
    rng: &mut ChaCha8Rng,
    count: &mut usize,
    inner_notify_stop: Arc<Notify>,
    lines: &Vec<Line>,
) {
    for line in lines {
        rng_check_line(&cfg, rng, &line);
        *count += 1;
        if *count % 1000 == 0 {
            println!("{}", count);
        }
        if *count >= cfg.lines_amount {
            inner_notify_stop.notify_one();
        }
    }
}

pub fn vec_check_lines(
    cfg: &TestCfg,
    count: &mut usize,
    inner_notify_stop: Arc<Notify>,
    lines: &Vec<Line>,
) {
    for line in lines {
        assert_eq!(
            match &line.line {
                Some(v) => Some(v.replace("\r", "")),
                None => None,
            },
            Some(cfg.lines[*count].clone())
        );
        *count += 1;
        if *count % 1000 == 0 {
            println!("{}", count);
        }
        if *count >= cfg.lines.len() {
            inner_notify_stop.notify_one();
        }
    }
}

pub fn check_line_lengths(
    cfg: &TestCfg,
    count: &mut usize,
    inner_notify_stop: Arc<Notify>,
    lines: &Vec<Line>,
) {
    for line in lines {
        match &line.line {
            Some(l) => {
                assert!(
                    l.replace("\r", "").len() >= cfg.string_len_range.start
                        && l.replace("\r", "").len() < cfg.string_len_range.end
                )
            }
            None => (),
        }
        *count += 1;
        if *count % 1000 == 0 {
            println!("{}", count);
        }
        if *count >= cfg.lines_amount {
            inner_notify_stop.notify_one();
        }
    }
}

async fn ingest_error_return() -> Option<Result<Response<Body>, IngestError>> {
    // what in the world is this?
    Some(Err(IngestError::IoError(Error::new(
        ErrorKind::Other,
        "this is a test error; arrr I'm a pirate",
    ))))
}

pub async fn mock_server(cfg: TestCfg) {
    println!("starting mock server");
    let rng = Mutex::new(get_rng(&cfg));
    let count = Arc::new(Mutex::new(0));
    let proc_count = count.clone();
    let ref_count = count.clone();
    let notify_stop = Arc::new(Notify::new());
    let inner_notify_stop = notify_stop.clone();
    let inner_cfg = cfg.clone();
    let inner_test_type = cfg.test_type.clone();
    let (server, _, shutdown_handle) = http_ingester_with_processors(
        cfg.addr,
        None,
        // executed directly after receiving request
        Box::new(move |_| {
            if let TestType::Retry = inner_test_type {
                // error on every second request
                if *ref_count.lock().unwrap() % 2 == 0 {
                    *ref_count.lock().unwrap() += 1;
                    return Some(Box::pin(ingest_error_return()));
                }
            }
            None
        }),
        // executed after request has been accepted and parsed
        Box::new(move |body| {
            match inner_cfg.test_type {
                TestType::Vec => vec_check_lines(
                    &inner_cfg,
                    &mut proc_count.lock().unwrap(),
                    inner_notify_stop.clone(),
                    &body.lines,
                ),
                TestType::Rng => rng_check_lines(
                    &inner_cfg,
                    &mut rng.lock().unwrap(),
                    &mut proc_count.lock().unwrap(),
                    inner_notify_stop.clone(),
                    &body.lines,
                ),
                TestType::LineLength => check_line_lengths(
                    &inner_cfg,
                    &mut proc_count.lock().unwrap(),
                    inner_notify_stop.clone(),
                    &body.lines,
                ),
                TestType::Retry => {
                    println!("{}", *proc_count.lock().unwrap());
                    assert_eq!(
                        body.lines[0].line,
                        Some(format!("Critical docker_logdna error: Failed to send line to logdna ({}/5 retries): connection closed before message completed", *proc_count.lock().unwrap() / 2))
                    );
                    if *proc_count.lock().unwrap() == 11 {
                        inner_notify_stop.notify_one();
                    }
                    *proc_count.lock().unwrap() += 1;
                }
            }

            Some(Box::pin(sleep(cfg.ingester_delay)))
        }),
    );

    tokio::select! {
        _ = run_server(notify_stop, server, shutdown_handle) => {},
        _ = timeout(cfg.clone(), count, cfg.lines_amount) => {},
    };
}

async fn run_server(
    notify_stop: Arc<Notify>,
    server: impl Future<Output = std::result::Result<(), IngestError>>,
    shutdown_handle: impl FnOnce(),
) {
    tokio::join!(
        async {
            notify_stop.notified().await;
            println!("Shutting down mock server");
            shutdown_handle();
        },
        server
    )
    .1
    .unwrap();
}

async fn timeout(cfg: TestCfg, count: Arc<Mutex<usize>>, lines_amount: usize) {
    sleep(cfg.server_timeout).await;
    panic!(
        "the mock server has timed out at {}/{} lines",
        count.lock().unwrap(),
        lines_amount
    );
}

pub async fn mock_client(cfg: TestCfg) {
    let delta = Duration::from_millis((1000.0 / cfg.lines_per_sec) as u64);
    let mut rng = get_rng(&cfg);

    let start = SystemTime::now();
    let mut slow_streak: u64 = 0;
    tokio::select!(
        _ = async {
            // TODO: ctrl+c doesn't seem to work with docker
            signal::ctrl_c().await.unwrap();
            println!("Shutting down mock client");
        } => {},
        _ = async {
            for i in 0..cfg.lines_amount {
                println!("{}", get_str(&cfg, &mut rng));
                let target_time = delta * (i as u32 + 1);
                let cur_time = SystemTime::now().duration_since(start).unwrap();
                if cur_time <= target_time {
                    slow_streak = 0;
                    sleep(target_time - cur_time).await;
                } else {
                    slow_streak += 1;
                    if cfg.max_slow_streak != 0 {
                        assert!(slow_streak <= cfg.max_slow_streak);
                    }
                }
            }
        } => {},
    );
}

pub fn load_cfg() -> TestCfg {
    TestCfg {
        addr: std::env::var("MOCK_ADDRESS")
            .unwrap()
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap(),
        seed: std::env::var("MOCK_SEED").unwrap().parse::<u64>().unwrap(),
        lines: Vec::new(),
        test_type: TestType::Rng,

        lines_per_sec: std::env::var("MOCK_LINES_PER_SEC")
            .unwrap()
            .parse::<f64>()
            .unwrap(),
        lines_amount: std::env::var("MOCK_LINES_AMOUNT")
            .unwrap()
            .parse::<usize>()
            .unwrap(),
        max_slow_streak: std::env::var("MOCK_MAX_SLOW_STREAK")
            .unwrap()
            .parse::<u64>()
            .unwrap(),

        char_range: std::env::var("MOCK_CHAR_MIN")
            .unwrap()
            .parse::<u8>()
            .unwrap()
            ..std::env::var("MOCK_CHAR_MAX")
                .unwrap()
                .parse::<u8>()
                .unwrap(),
        string_len_range: std::env::var("MOCK_STR_LEN_MIN")
            .unwrap()
            .parse::<usize>()
            .unwrap()
            ..std::env::var("MOCK_STR_LEN_MAX")
                .unwrap()
                .parse::<usize>()
                .unwrap(),

        ingester_delay: Duration::from_millis(
            std::env::var("MOCK_INGESTER_DELAY")
                .unwrap()
                .parse::<u64>()
                .unwrap(),
        ),
        server_timeout: Duration::from_millis(
            std::env::var("MOCK_SERVER_TIMEOUT")
                .unwrap()
                .parse::<u64>()
                .unwrap(),
        ),
    }
}

pub fn make_panic_exit() {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));
}
