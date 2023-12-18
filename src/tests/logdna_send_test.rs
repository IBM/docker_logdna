use std::time::Duration;

use logdna_mock::{mock_server, TestCfg, TestType};
use serde_json::from_str;
use tokio::time::sleep;

use crate::{
    get_container::get_client,
    logdna_send::send_critical_error,
    logdna_send::{create_logdna_line, send_lines},
    structs::{Config, Container, StartRequest},
};

#[tokio::test]
async fn send_lines_test() {
    let cfg = TestCfg::new_vec(
        "127.0.0.1:9000".parse().unwrap(),
        vec![
            "For whom, for what was that bird singing?".to_string(),
            "no mate, no rival was watching it.".to_string(),
            "What made it sit at the edge of the lonely wood and pour its music into nothingness?"
                .to_string(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        ],
    );
    tokio::select!(
        _ = send_lines_test_sender() => {},
        // check labels and such can't be checked because of private fields in logdna server mocking library
        _ = mock_server(cfg) => {},
    );
}

async fn send_lines_test_sender() {
    let start_request: StartRequest =
        from_str(r#"{"File": "","Info": {"ContainerID": "","ContainerLabels": {"test_label": "test_value", "a": "b"}}}"#).unwrap();
    let config = Config {
        hostname: "test-i-dont-care".to_string(),
        logdna_host: "127.0.0.1:9000".to_string(),
        api_key: "my-cool-api-key".to_string(),
        ip: Some("111.111.111.111".to_string()),
        mac: Some("0014a541616b".to_string()),
        tags: "something,some,asfd".to_string(),
        app: "app".to_string(),
        level: None,
        // max_length doesn't get enforced in send_lines function
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
    send_lines(
        &container,
        vec![create_logdna_line(
            &container,
            "For whom, for what was that bird singing?".to_string(),
        )
        .await
        .unwrap()],
        false,
    )
    .await
    .unwrap();
    send_lines(
        &container,
        vec![
            create_logdna_line(&container, "no mate, no rival was watching it.".to_string())
                .await
                .unwrap(),
        ],
        false,
    )
    .await
    .unwrap();
    send_lines(
        &container,
        vec![create_logdna_line(
            &container,
            "What made it sit at the edge of the lonely wood and pour its music into nothingness?"
                .to_string(),
        )
        .await
        .unwrap()],
        false,
    )
    .await
    .unwrap();
    send_lines(&container, vec![create_logdna_line(&container, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()).await.unwrap()], false).await.unwrap();
    sleep(Duration::from_millis(1000)).await;
    panic!("server didn't receive message fast enough");
}

#[tokio::test]
async fn send_critical_error_test() {
    let cfg = TestCfg::new_vec(
        "127.0.0.1:9001".parse().unwrap(),
        vec![
            "Critical docker_logdna error: What a terrible fate.".to_string(),
            "Critical docker_logdna error: talking by installments".to_string(),
            "Critical docker_logdna error: Desire was thoughtcrime.".to_string(),
        ],
    );
    tokio::select!(
        _ = send_critical_error_test_sender() => {},
        _ = mock_server(cfg) => {},
    );
}

async fn send_critical_error_test_sender() {
    let start_request: StartRequest =
        from_str(r#"{"File": "","Info": {"ContainerID": ""}}"#).unwrap();
    let config = Config {
        hostname: "test-i-dont-care".to_string(),
        logdna_host: "127.0.0.1:9001".to_string(),
        api_key: "my-cool-api-key".to_string(),
        ip: None,
        mac: None,
        tags: "".to_string(),
        app: "app".to_string(),
        level: None,
        max_length: 0,
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
    send_critical_error(&container, "What a terrible fate.".to_string()).await;
    send_critical_error(&container, "talking by installments".to_string()).await;
    send_critical_error(&container, "Desire was thoughtcrime.".to_string()).await;
    sleep(Duration::from_millis(1000)).await;
    panic!("server didn't receive message fast enough");
}

#[tokio::test]
async fn send_lines_retries_test() {
    let mut cfg = TestCfg::new_vec("127.0.0.1:9010".parse().unwrap(), vec![]);
    cfg.test_type = TestType::Retry;
    tokio::select!(
        _ = send_lines_retries_test_sender() => {},
        _ = mock_server(cfg) => {},
    );
}

async fn send_lines_retries_test_sender() {
    let start_request: StartRequest =
        from_str(r#"{"File": "","Info": {"ContainerID": "","ContainerLabels": {"test_label": "test_value", "a": "b"}}}"#).unwrap();
    let config = Config {
        hostname: "test-i-dont-care".to_string(),
        logdna_host: "127.0.0.1:9010".to_string(),
        api_key: "my-cool-api-key".to_string(),
        ip: Some("111.111.111.111".to_string()),
        mac: Some("0014a541616b".to_string()),
        tags: "something,some,asfd".to_string(),
        app: "app".to_string(),
        level: None,
        // max_length doesn't get enforced in send_lines function
        max_length: 100,
        for_mock_server: true,
        flush_interval: Duration::from_millis(250),
        max_buffer_size: 2097152,
        http_client_timeout: Duration::from_millis(30000),
        // test this
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
    send_lines(
        &container,
        vec![create_logdna_line(&container, "".to_string())
            .await
            .unwrap()],
        false,
    )
    .await
    .unwrap();
    sleep(Duration::from_millis(1000)).await;
    panic!("server didn't receive message fast enough");
}

#[tokio::test]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value: ()")]
async fn fail_immediately_test() {
    let start_request: StartRequest =
        from_str(r#"{"File": "","Info": {"ContainerID": "","ContainerLabels": {"test_label": "test_value", "a": "b"}}}"#).unwrap();
    let config = Config {
        hostname: "test-i-dont-care".to_string(),
        // don't start this server
        logdna_host: "127.0.0.1:9069".to_string(),
        api_key: "my-cool-api-key".to_string(),
        ip: Some("111.111.111.111".to_string()),
        mac: Some("0014a541616b".to_string()),
        tags: "something,some,asfd".to_string(),
        app: "app".to_string(),
        level: None,
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

    send_lines(
        &container,
        vec![create_logdna_line(&container, "".to_string())
            .await
            .unwrap()],
        true,
    )
    .await
    .unwrap();
}
