use crate::event::{DownloadEvent, DownloadState};
use std::fs::{self, OpenOptions};
use std::io::Write;
use tokio::io::AsyncReadExt;

pub async fn download<F: Fn(u64)>(event: &mut DownloadEvent, on_progress: F) {
    event.start();

    let progress = if let DownloadState::Downloading { progress } = event.state {
        progress
    } else {
        return;
    };

    let client = match reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            event.fail(format!("Failed to create HTTP client: {}", e));
            return;
        }
    };

    let mut request = client.get(&event.url);
    if progress > 0 {
        request = request.header("Range", format!("bytes={}-", progress));
    }

    let response = match request.send().await {
        Ok(resp) => {
            let status = resp.status();
            if status != 200 && status != 206 {
                event.fail(format!("HTTP error: {}", status));
                return;
            }
            resp
        }
        Err(e) => {
            event.fail(format!("Request failed: {}", e));
            return;
        }
    };

    // 获取文件总大小
    if event.total_size == 0 {
        if let Some(content_range) = response.headers().get("Content-Range") {
            if let Ok(range_str) = content_range.to_str() {
                if let Some(total_str) = range_str.split('/').nth(1) {
                    if let Ok(total) = total_str.parse::<u64>() {
                        event.total_size = total;
                    }
                }
            }
        }
        if event.total_size == 0 {
            if let Some(content_length) = response.headers().get("Content-Length") {
                if let Ok(len_str) = content_length.to_str() {
                    if let Ok(len) = len_str.parse::<u64>() {
                        event.total_size = progress + len;
                    }
                }
            }
        }
    }

    let mut file = match OpenOptions::new()
        .create(true)
        .append(progress > 0)
        .write(true)
        .open(&event.temp_file)
    {
        Ok(f) => f,
        Err(e) => {
            event.fail(format!("Cannot open temp file: {}", e));
            return;
        }
    };

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(data) => {
                if let Err(e) = file.write_all(&data) {
                    event.fail(format!("Write error: {}", e));
                    return;
                }
                if let DownloadState::Downloading { ref mut progress } = event.state {
                    *progress += data.len() as u64;
                    on_progress(*progress);
                }
            }
            Err(e) => {
                event.fail(format!("Read error: {}", e));
                return;
            }
        }
    }

    drop(file);

    if let Err(e) = fs::rename(&event.temp_file, &event.dest) {
        event.fail(format!("Rename error: {}", e));
        return;
    }

    event.complete();
}