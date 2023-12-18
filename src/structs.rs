use std::{collections::HashMap, time::Duration};

use logdna_client::client::Client;
use serde::Deserialize;
use tokio::sync::Notify;

mod entry {
    // somehow the name is not entry.rs
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}
pub type LogEntry = entry::LogEntry;

pub struct CloseNotify {
    // sent when docker sends close request via http to /LogDriver.StopLogging
    pub requested_to_close: Notify,
    // sent when all messages have been read from the pipe and docker can close it, causing an EOF
    pub let_docker_close_pipe: Notify,
}
impl CloseNotify {
    pub fn new() -> CloseNotify {
        CloseNotify {
            requested_to_close: Notify::new(),
            let_docker_close_pipe: Notify::new(),
        }
    }
}

// single source of truth for one connection to a container
pub struct Container {
    pub start_request: StartRequest,
    pub config: Config,
    pub client: Client,
}

// what the user defines with --log-opt
#[derive(Debug, PartialEq)]
pub struct Config {
    // use machine hostname if undefined
    pub hostname: String,
    pub logdna_host: String,
    pub api_key: String,

    pub ip: Option<String>,
    pub mac: Option<String>,

    // optional
    pub tags: String,

    // use container name if undefined
    pub app: String,
    // one of TRACE, DEBUG, INFO, WARN, ERROR, FATAL
    // or left undefined
    pub level: Option<String>,

    // max log line length
    // max_length 0 -> don't limit length
    // undefined -> max_length of 8192
    pub max_length: usize,

    // uses http rather than https to connect to logdna
    // uses /logd/agent rather than /logs/ingest as ingest endpoint
    // default to false
    pub for_mock_server: bool,

    // after what time to flush log lines to logdna
    // when undefined send after 250ms
    pub flush_interval: Duration,
    // after how many bytes of log to flush log lines to logdna
    // when undefined send after 2MB
    pub max_buffer_size: usize,
    // logdna http request timeout
    // when undefined time out after 30sec
    pub http_client_timeout: Duration,
    // how often to retry sending lines to logdna
    // when undefined retry five times
    pub max_request_retry: usize,
}

// what docker sends via http to /LogDriver.StartLogging
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartRequest {
    #[serde(rename = "File")]
    pub file: String,
    #[serde(rename = "Info")]
    pub info: StartInfo,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartInfo {
    #[serde(rename = "Config")]
    pub config: Option<HashMap<String, String>>,
    #[serde(rename = "ContainerID")]
    pub container_id: String,
    #[serde(rename = "ContainerName")]
    pub container_name: Option<String>,
    #[serde(rename = "ContainerEntrypoint")]
    pub container_entry_point: Option<String>,
    #[serde(rename = "ContainerArgs")]
    pub container_args: Option<Vec<String>>,
    #[serde(rename = "ContainerImageID")]
    pub container_image_id: Option<String>,
    #[serde(rename = "ContainerImageName")]
    pub container_image_name: Option<String>,
    #[serde(rename = "ContainerCreated")]
    pub container_created: Option<String>,
    #[serde(rename = "ContainerEnv")]
    pub container_env: Option<Vec<String>>,
    #[serde(rename = "ContainerLabels")]
    pub container_labels: Option<HashMap<String, String>>,
    #[serde(rename = "LogPath")]
    pub log_path: Option<String>,
    #[serde(rename = "DaemonName")]
    pub daemon_name: Option<String>,
}

// what docker sends via http to /LogDriver.StopLogging
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct StopRequest {
    #[serde(rename = "File")]
    pub file: String,
}
