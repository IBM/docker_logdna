use std::time::Duration;

use crate::{
    get_container::{get_config, get_container},
    structs::{Config, StartRequest},
};
use gethostname::gethostname;
use serde_json::from_str;

#[tokio::test]
async fn get_config_no_cfg_test() {
    let start_request: StartRequest = from_str(r#"
{
    "File": "/run/docker/logging/c2ba1c15360342684b551b46908563d58f66b588251743c5b372962a4c76d534",
    "Info": {
        "ContainerID": "ea61498dafdcc2aaed3e301772124617e8940afc304270ec6ae1653b5680abe1",
        "ContainerName": "/bold_mendeleev",
        "ContainerEntrypoint": "bash",
        "ContainerArgs": [],
        "ContainerImageID": "sha256:c31f65dd4cc97f21bd440df4b9b919a99e35a7ede602e8150f2314af1441a312",
        "ContainerImageName": "debian",
        "ContainerCreated": "2023-10-02T06:16:25.299394139Z",
        "ContainerEnv": [
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        ],
        "ContainerLabels": {},
        "LogPath": "",
        "DaemonName": "docker"
    }
}
    "#).unwrap();
    let expected_cfg = Err("The logdna logging driver needs a config.".to_string());
    assert_eq!(get_config(&start_request).await, expected_cfg);
}

#[tokio::test]
async fn get_config_no_logdna_host_test() {
    let start_request: StartRequest = from_str(r#"
{
    "File": "/run/docker/logging/c2ba1c15360342684b551b46908563d58f66b588251743c5b372962a4c76d534",
    "Info": {
        "Config": {
            "api_key": "something-cool"
        },
        "ContainerID": "ea61498dafdcc2aaed3e301772124617e8940afc304270ec6ae1653b5680abe1",
        "ContainerName": "/bold_mendeleev",
        "ContainerEntrypoint": "bash",
        "ContainerArgs": [],
        "ContainerImageID": "sha256:c31f65dd4cc97f21bd440df4b9b919a99e35a7ede602e8150f2314af1441a312",
        "ContainerImageName": "debian",
        "ContainerCreated": "2023-10-02T06:16:25.299394139Z",
        "ContainerEnv": [
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        ],
        "ContainerLabels": {},
        "LogPath": "",
        "DaemonName": "docker"
    }
}
    "#).unwrap();
    let expected_cfg =
        Err("The logdna logging driver config needs the 'logdna_host' field.".to_string());
    assert_eq!(get_config(&start_request).await, expected_cfg);
}

#[tokio::test]
async fn get_config_no_api_key_test() {
    let start_request: StartRequest = from_str(r#"
{
    "File": "/run/docker/logging/c2ba1c15360342684b551b46908563d58f66b588251743c5b372962a4c76d534",
    "Info": {
        "Config": {
            "logdna_host": "chris-besch.com"
        },
        "ContainerID": "ea61498dafdcc2aaed3e301772124617e8940afc304270ec6ae1653b5680abe1",
        "ContainerName": "/bold_mendeleev",
        "ContainerEntrypoint": "bash",
        "ContainerArgs": [],
        "ContainerImageID": "sha256:c31f65dd4cc97f21bd440df4b9b919a99e35a7ede602e8150f2314af1441a312",
        "ContainerImageName": "debian",
        "ContainerCreated": "2023-10-02T06:16:25.299394139Z",
        "ContainerEnv": [
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        ],
        "ContainerLabels": {},
        "LogPath": "",
        "DaemonName": "docker"
    }
}
    "#).unwrap();
    let expected_cfg =
        Err("The logdna logging driver config needs the 'api_key' field.".to_string());
    assert_eq!(get_config(&start_request).await, expected_cfg);
}

#[tokio::test]
async fn get_config_full_cfg_test() {
    let start_request: StartRequest = from_str(r#"
{
    "File": "/run/docker/logging/c2ba1c15360342684b551b46908563d58f66b588251743c5b372962a4c76d534",
    "Info": {
        "Config": {
            "hostname": "test-hostname",
            "logdna_host": "chris-besch.com",
            "api_key": "my-cool-api-key",
            "ip": "123.123.123.123",
            "mac": "0014a541616b",
            "tags": "i,like,cheese",
            "app": "my-fancy-app",
            "level": "FATAL",
            "max_length": "69",
            "for_mock_server": "true",
            "flush_interval": "12344",
            "max_buffer_size": "55543",
            "http_client_timeout": "2845",
            "max_request_retry": "45"
        },
        "ContainerID": "ea61498dafdcc2aaed3e301772124617e8940afc304270ec6ae1653b5680abe1",
        "ContainerName": "/bold_mendeleev",
        "ContainerEntrypoint": "bash",
        "ContainerArgs": [],
        "ContainerImageID": "sha256:c31f65dd4cc97f21bd440df4b9b919a99e35a7ede602e8150f2314af1441a312",
        "ContainerImageName": "debian",
        "ContainerCreated": "2023-10-02T06:16:25.299394139Z",
        "ContainerEnv": [
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        ],
        "ContainerLabels": {},
        "LogPath": "",
        "DaemonName": "docker"
    }
}
    "#).unwrap();
    let expected_cfg = Ok(Config {
        hostname: "test-hostname".to_string(),
        logdna_host: "chris-besch.com".to_string(),
        api_key: "my-cool-api-key".to_string(),

        ip: Some("123.123.123.123".to_string()),
        mac: Some("0014a541616b".to_string()),

        tags: "i,like,cheese".to_string(),

        app: "my-fancy-app".to_string(),
        level: Some("FATAL".to_string()),

        max_length: 69,

        for_mock_server: true,
        flush_interval: Duration::from_millis(12344),
        max_buffer_size: 55543,
        http_client_timeout: Duration::from_millis(2845),
        max_request_retry: 45,
    });
    assert_eq!(get_config(&start_request).await, expected_cfg);
}

#[tokio::test]
async fn get_config_slim_cfg_test() {
    let start_request: StartRequest = from_str(r#"
{
    "File": "/run/docker/logging/c2ba1c15360342684b551b46908563d58f66b588251743c5b372962a4c76d534",
    "Info": {
        "Config": {
            "logdna_host": "chris-besch.com",
            "api_key": "my-cool-api-key"
        },
        "ContainerID": "ea61498dafdcc2aaed3e301772124617e8940afc304270ec6ae1653b5680abe1",
        "ContainerName": "/bold_mendeleev",
        "ContainerEntrypoint": "bash",
        "ContainerArgs": [],
        "ContainerImageID": "sha256:c31f65dd4cc97f21bd440df4b9b919a99e35a7ede602e8150f2314af1441a312",
        "ContainerImageName": "debian",
        "ContainerCreated": "2023-10-02T06:16:25.299394139Z",
        "ContainerEnv": [
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        ],
        "ContainerLabels": {},
        "LogPath": "",
        "DaemonName": "docker"
    }
}
    "#).unwrap();
    let expected_cfg = Ok(Config {
        hostname: gethostname().to_str().unwrap().to_string(),
        logdna_host: "chris-besch.com".to_string(),
        api_key: "my-cool-api-key".to_string(),

        ip: None,
        mac: None,

        tags: "".to_string(),

        app: "/bold_mendeleev".to_string(),
        level: None,

        max_length: 8192,

        for_mock_server: false,
        flush_interval: Duration::from_millis(250),
        max_buffer_size: 2097152,
        http_client_timeout: Duration::from_millis(30000),
        max_request_retry: 5,
    });
    assert_eq!(get_config(&start_request).await, expected_cfg);
}

#[tokio::test]
async fn get_config_as_small_as_possible() {
    let start_request: StartRequest = from_str(
        r#"
{
    "File": "/run/docker/logging/c2ba1c15360342684b551b46908563d58f66b588251743c5b372962a4c76d534",
    "Info": {
        "Config": {
            "logdna_host": "chris-besch.com",
            "api_key": "my-cool-api-key"
        },
        "ContainerID": "ea61498dafdcc2aaed3e301772124617e8940afc304270ec6ae1653b5680abe1"
    }
}
    "#,
    )
    .unwrap();
    let expected_cfg = Ok(Config {
        hostname: gethostname().to_str().unwrap().to_string(),
        logdna_host: "chris-besch.com".to_string(),
        api_key: "my-cool-api-key".to_string(),

        ip: None,
        mac: None,

        tags: "".to_string(),

        app: "ea61498dafdcc2aaed3e301772124617e8940afc304270ec6ae1653b5680abe1".to_string(),
        level: None,

        max_length: 8192,

        for_mock_server: false,
        flush_interval: Duration::from_millis(250),
        max_buffer_size: 2097152,
        http_client_timeout: Duration::from_millis(30000),
        max_request_retry: 5,
    });
    assert_eq!(get_config(&start_request).await, expected_cfg);
}

#[tokio::test]
async fn get_container_test() {
    let start_request: StartRequest = from_str(r#"
{
    "File": "/run/docker/logging/c2ba1c15360342684b551b46908563d58f66b588251743c5b372962a4c76d534",
    "Info": {
        "Config": {
            "hostname": "test-hostname",
            "logdna_host": "chris-besch.com",
            "api_key": "my-cool-api-key",
            "ip": "123.123.123.123",
            "mac": "0014a541616b",
            "tags": "i,like,cheese",
            "app": "my-fancy-app",
            "level": "FATAL",
            "max_length": "69",
            "for_mock_server": "true",
            "flush_interval": "12344",
            "max_buffer_size": "55543",
            "http_client_timeout": "2845",
            "max_request_retry": "45"
        },
        "ContainerID": "ea61498dafdcc2aaed3e301772124617e8940afc304270ec6ae1653b5680abe1",
        "ContainerName": "/bold_mendeleev",
        "ContainerEntrypoint": "bash",
        "ContainerArgs": [],
        "ContainerImageID": "sha256:c31f65dd4cc97f21bd440df4b9b919a99e35a7ede602e8150f2314af1441a312",
        "ContainerImageName": "debian",
        "ContainerCreated": "2023-10-02T06:16:25.299394139Z",
        "ContainerEnv": [
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        ],
        "ContainerLabels": {},
        "LogPath": "",
        "DaemonName": "docker"
    }
}
    "#).unwrap();
    let expected_cfg = Config {
        hostname: "test-hostname".to_string(),
        logdna_host: "chris-besch.com".to_string(),
        api_key: "my-cool-api-key".to_string(),

        ip: Some("123.123.123.123".to_string()),
        mac: Some("0014a541616b".to_string()),

        tags: "i,like,cheese".to_string(),

        app: "my-fancy-app".to_string(),
        level: Some("FATAL".to_string()),

        max_length: 69,

        for_mock_server: true,
        flush_interval: Duration::from_millis(12344),
        max_buffer_size: 55543,
        http_client_timeout: Duration::from_millis(2845),
        max_request_retry: 45,
    };
    let container = get_container(start_request.clone()).await.unwrap();
    assert_eq!(container.config, expected_cfg);
    assert_eq!(container.start_request, start_request);
    // container.client can't be tested because it's a bunch of private fields in the logdna client
    // library
}
