use logdna_client::body::{IngestBody, KeyValueMap, Line};

use crate::structs::Container;

#[cfg(test)]
#[path = "./tests/logdna_send_test.rs"]
mod logdna_send_test;

pub async fn create_logdna_line(container: &Container, line: String) -> Result<Line, String> {
    let mut labels = KeyValueMap::new();
    match &container.start_request.info.container_labels {
        Some(v) => {
            for (key, value) in v {
                labels = labels.add(key, value);
            }
        }
        None => (),
    }

    let mut logdna_line = Line::builder()
        .line(line)
        .app(&container.config.app)
        .labels(labels);

    if let Some(ref level) = container.config.level {
        logdna_line = logdna_line.level(level);
    }

    match logdna_line.build() {
        Ok(v) => Ok(v),
        Err(e) => Err(format!("Failed to build logdna line: {}", e)),
    }
}

// when one_shot is true don't retry and fail immediately
pub async fn send_lines(container: &Container, lines: Vec<Line>, one_shot: bool) -> Result<(), ()> {
    let max_retries = match one_shot {
        true => 0,
        false => container.config.max_request_retry,
    };
    let body = IngestBody::new(lines);
    let mut retries: usize = 0;
    while let Err(e) = container.client.send(&body).await {
        send_critical_error(
            container,
            format!(
                "Failed to send line to logdna ({}/{} retries): {}",
                retries, max_retries, e
            ),
        )
        .await;
        retries += 1;
        if retries > max_retries {
            return Err(());
        }
    }
    Ok(())
}

pub async fn send_critical_error(container: &Container, err: String) {
    let err = format!("Critical docker_logdna error: {}", err);
    println!("{}", err);

    let logdna_line = match Line::builder()
        .line(err)
        .app(&container.config.app)
        .level("FATAL")
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            println!(
                "Super Critical Error: failed to build logdna critical error line: {}",
                e
            );
            return;
        }
    };

    let body = IngestBody::new(vec![logdna_line]);
    match container.client.send(body).await {
        Ok(_) => (),
        Err(e) => {
            println!(
                "Super Critical Error: failed to send critical error line: {}",
                e
            );
        }
    }
}
