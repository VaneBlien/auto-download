use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct ProgressManager {
    multi: MultiProgress,
    bars: Mutex<HashMap<String, ProgressBar>>,
}

impl ProgressManager {
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            bars: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_bar(&self, url: &str, total_size: u64) {
        let pb = self.multi.add(ProgressBar::new(total_size));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );
        pb.set_prefix(url.to_string());
        let mut bars = self.bars.lock().unwrap();
        bars.insert(url.to_string(), pb);
    }

    pub fn update(&self, url: &str, progress: u64) {
        let bars = self.bars.lock().unwrap();
        if let Some(pb) = bars.get(url) {
            pb.set_position(progress);
        }
    }

    pub fn finish(&self, url: &str, message: &str) {
        let mut bars = self.bars.lock().unwrap();
        if let Some(pb) = bars.remove(url) {
            pb.finish_with_message(message.to_string());
        }
    }

    pub fn error(&self, url: &str, message: &str) {
        let mut bars = self.bars.lock().unwrap();
        if let Some(pb) = bars.remove(url) {
            pb.finish_with_message(format!("❌ {}", message));
        }
    }
}