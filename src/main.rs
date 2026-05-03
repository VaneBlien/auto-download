mod downloader;
mod event;
mod reporter;
mod worker;

use crate::event::DownloadEvent;
use crate::reporter::ProgressManager;
use clap::Parser;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(name = "auto-download")]
#[command(about = "A concurrent downloader with resume support", long_about = None)]
struct Cli {
    #[arg(required = true, num_args = 1..)]
    urls: Vec<String>,

    #[arg(short = 't', long = "threads", default_value = "4")]
    threads: usize,

    #[arg(short = 'o', long = "output-dir", default_value = ".")]
    output_dir: PathBuf,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = std::fs::create_dir_all(&cli.output_dir) {
        eprintln!("Failed to create output directory: {}", e);
        std::process::exit(1);
    }

    let url_count = cli.urls.len();
    let worker_count = usize::min(url_count, cli.threads);
    println!("Starting {} worker(s) for {} URL(s)", worker_count, url_count);

    let (task_tx, task_rx) = mpsc::channel::<DownloadEvent>();
    let (result_tx, result_rx) = mpsc::channel::<DownloadEvent>();

    let shared_rx = Arc::new(Mutex::new(task_rx));
    let progress_mgr = Arc::new(ProgressManager::new());

    let mut workers = Vec::new();
    for id in 0..worker_count {
        let rx = Arc::clone(&shared_rx);
        let tx = result_tx.clone();
        let retry_tx = task_tx.clone();
        let pm = Arc::clone(&progress_mgr);
        workers.push(worker::spawn_worker(id, rx, tx, retry_tx, pm));
    }

    for url in &cli.urls {
        let filename = url.split('/').last().unwrap_or("downloaded_file");
        let dest = cli
            .output_dir
            .join(filename)
            .to_string_lossy()
            .to_string();
        let event = DownloadEvent::new(url.clone(), dest);
        println!("Enqueued: {}", url);
        task_tx.send(event).expect("Failed to enqueue task");
    }

    drop(result_tx);

    let mut completed = 0;
    let mut failed = 0;

    while let Ok(event) = result_rx.recv() {
        match event.state {
            event::DownloadState::Completed => completed += 1,
            event::DownloadState::Failed { .. } => failed += 1,
            _ => {}
        }
    }

    drop(task_tx);
    for handle in workers {
        let _ = handle.await;
    }

    println!("--- Done ---");
    println!("Completed: {}, Failed: {}", completed, failed);
}