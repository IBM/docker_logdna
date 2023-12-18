use std::time::Duration;

use gethostname::gethostname;
use logdna_client::params::Params;
use logdna_client::request::RequestTemplate;
use logdna_client::{client::Client, request::Schema};

use crate::structs::{Config, Container, StartRequest};

#[cfg(test)]
#[path = "./tests/get_container_test.rs"]
mod get_container_test;

pub async fn get_container(start_request: StartRequest) -> Result<Container, String> {
    let config = get_config(&start_request).await?;
    let client = get_client(&config).await?;

    let container = Container {
        start_request,
        config,
        client,
    };
    Ok(container)
}

async fn get_config(start_request: &StartRequest) -> Result<Config, String> {
    let cfg = match start_request.info.config {
        Some(ref cfg) => cfg,
        None => return Err(format!("The logdna logging driver needs a config.")),
    };

    let hostname = match cfg.get("hostname") {
        Some(v) => v.clone(),
        None => match gethostname().to_str() {
            Some(h) => h,
            None => {
                println!("Error: no hostname found");
                "err-no-hostname-found"
            }
        }
        .to_string(),
    };

    let logdna_host = match cfg.get("logdna_host") {
        Some(v) => v.clone(),
        None => {
            return Err(format!(
                "The logdna logging driver config needs the 'logdna_host' field."
            ))
        }
    };

    let api_key = match cfg.get("api_key") {
        Some(v) => v.clone(),
        None => {
            return Err(format!(
                "The logdna logging driver config needs the 'api_key' field."
            ))
        }
    };

    let ip = cfg.get("ip").cloned();
    let mac = cfg.get("mac").cloned();
    let tags = match cfg.get("tags") {
        Some(v) => v.clone(),
        None => "".to_string(),
    };
    let app = match cfg.get("app") {
        Some(v) => v.clone(),
        None => format!(
            "{}",
            match start_request.info.container_name {
                Some(ref c) => c.clone(),
                None => start_request.info.container_id.clone(),
            }
        ),
    };
    let level = cfg.get("level").cloned();

    let max_length = match cfg.get("max_length") {
        Some(v) => match v.parse::<usize>() {
            Ok(u) => u,
            Err(e) => return Err(format!("failed to parse max_length: {}", e)),
        },
        None => 8192,
    };
    let for_mock_server = match cfg.get("for_mock_server") {
        Some(v) => match v.parse::<bool>() {
            Ok(u) => u,
            Err(e) => return Err(format!("failed to parse for_mock_server: {}", e)),
        },
        None => false,
    };
    let flush_interval = match cfg.get("flush_interval") {
        Some(v) => match v.parse::<u64>() {
            Ok(u) => Duration::from_millis(u),
            Err(e) => return Err(format!("failed to parse flush_interval: {}", e)),
        },
        None => Duration::from_millis(250),
    };
    let max_buffer_size = match cfg.get("max_buffer_size") {
        Some(v) => match v.parse::<usize>() {
            Ok(u) => u,
            Err(e) => return Err(format!("failed to parse max_buffer_size: {}", e)),
        },
        None => 2097152,
    };
    let http_client_timeout = match cfg.get("http_client_timeout") {
        Some(v) => match v.parse::<u64>() {
            Ok(u) => Duration::from_millis(u),
            Err(e) => return Err(format!("failed to parse http_client_timeout: {}", e)),
        },
        None => Duration::from_millis(30000),
    };
    let max_request_retry = match cfg.get("max_request_retry") {
        Some(v) => match v.parse::<usize>() {
            Ok(u) => u,
            Err(e) => return Err(format!("failed to parse max_request_retry: {}", e)),
        },
        None => 5,
    };

    let config = Config {
        hostname,
        logdna_host,
        api_key,
        ip,
        mac,
        tags,
        app,
        level,
        max_length,
        for_mock_server,
        flush_interval,
        max_buffer_size,
        http_client_timeout,
        max_request_retry,
    };
    Ok(config)
}

pub async fn get_client(config: &Config) -> Result<Client, String> {
    let mut params = Params::builder();
    params.hostname(config.hostname.clone());
    params.tags(config.tags.clone());

    match config.ip {
        Some(ref ip) => params.ip(ip.clone()),
        None => &mut params,
    };
    match config.mac {
        Some(ref mac) => params.mac(mac.clone()),
        None => &mut params,
    };
    let params = match params.build() {
        Ok(v) => v,
        Err(e) => return Err(format!("failed to build logdna params: {}", e)),
    };

    let template = match RequestTemplate::builder()
        .params(params)
        .host(config.logdna_host.clone())
        .api_key(config.api_key.clone())
        // /logs/agent is required for the mocking server to accept any requests
        // it is also used by the logdna agent v2
        // /logs/ingest is the default for the rust logdna client and listed in
        // [the api docs](https://docs.mezmo.com/log-analysis-api)
        .endpoint(match config.for_mock_server {
            true => "/logs/agent",
            false => "/logs/ingest",
        })
        // needs to be set to http for the mocking server to understand the requests
        .schema(match config.for_mock_server {
            true => Schema::Http,
            false => Schema::Https,
        })
        // .schema(Schema::Http)
        .build()
    {
        Ok(v) => v,
        Err(e) => return Err(format!("failed to build logdna request template: {}", e)),
    };

    // require_tls needs to be set to Some(false) for http to be used (required for the mocking server)
    let mut client = Client::new(template, Some(!config.for_mock_server));
    client.set_timeout(config.http_client_timeout);
    Ok(client)
}
