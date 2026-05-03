use crate::downloader;
use crate::event::{DownloadEvent, DownloadState};
use crate::reporter::ProgressManager;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub async fn run_worker(
    id: usize,
    rx: Arc<Mutex<Receiver<DownloadEvent>>>,
    result_tx: Sender<DownloadEvent>,
    task_tx: Sender<DownloadEvent>,
    progress_mgr: Arc<ProgressManager>,
) {
    println!("[Worker {}] started", id);

    loop {
        let mut event = {
            let rx = rx.lock().expect("Worker failed to lock receiver");
            match rx.recv() {
                Ok(event) => event,
                Err(_) => break,
            }
        };

        println!("[Worker {}] picked up: {}", id, event.url);

        progress_mgr.add_bar(&event.url, event.total_size);

        let url = event.url.clone();
        let pm = Arc::clone(&progress_mgr);

        downloader::download(&mut event, |progress| {
            pm.update(&url, progress);
        })
        .await;

        match &event.state {
            DownloadState::Completed => {
                progress_mgr.finish(&url, "✅");
            }
            DownloadState::Failed { error } => {
                progress_mgr.error(&url, error);
            }
            _ => {}
        }

        if !matches!(&event.state, DownloadState::Completed) {
            if event.retries < event.max_retries {
                println!(
                    "[Worker {}] retrying ({}/{}) in 1s: {}",
                    id, event.retries, event.max_retries, event.url
                );
                tokio::time::sleep(Duration::from_secs(1)).await;
                event.state = DownloadState::Pending;
                if let Err(e) = task_tx.send(event) {
                    eprintln!("[Worker {}] failed to requeue: {}", id, e);
                    break;
                }
                continue;
            }
        }

        if let Err(e) = result_tx.send(event) {
            eprintln!("[Worker {}] failed to send result: {}", id, e);
            break;
        }
    }

    println!("[Worker {}] shutting down", id);
}

pub fn spawn_worker(
    id: usize,
    rx: Arc<Mutex<Receiver<DownloadEvent>>>,
    result_tx: Sender<DownloadEvent>,
    task_tx: Sender<DownloadEvent>,
    progress_mgr: Arc<ProgressManager>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        run_worker(id, rx, result_tx, task_tx, progress_mgr).await;
    })
}