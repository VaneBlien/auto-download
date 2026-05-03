/// 下载事件的状态机
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadState {
    /// 等待被 worker 拾取
    Pending,
    /// 正在下载，携带已下载字节数
    Downloading { progress: u64 },
    /// 已暂停，携带断点位置
    Paused { progress: u64 },
    /// 下载完成
    Completed,
    /// 下载失败，携带错误信息
    Failed { error: String },
}

/// 一个下载事件
#[derive(Debug, Clone)]
pub struct DownloadEvent {
    /// 下载 URL
    pub url: String,
    /// 本地保存路径
    pub dest: String,
    /// 临时文件路径（用于断点续传）
    pub temp_file: String,
    /// 文件总大小（字节），初始为 0，收到响应头后更新
    pub total_size: u64,
    /// 当前状态
    pub state: DownloadState,
    /// 最大重试次数
    pub max_retries: u32,
    /// 当前已重试次数
    pub retries: u32,
}

impl DownloadEvent {
    /// 创建一个新的下载事件，初始状态为 Pending
    pub fn new(url: String, dest: String) -> Self {
        let temp_file = format!("{}.part", dest);
        Self {
            url,
            dest,
            temp_file,
            total_size: 0,
            state: DownloadState::Pending,
            max_retries: 3,
            retries: 0,
        }
    }

    /// 状态转换：Pending/Paused → Downloading
    pub fn start(&mut self) {
        match self.state {
            DownloadState::Pending | DownloadState::Paused { .. } => {
                let progress = self.get_existing_progress();
                self.state = DownloadState::Downloading { progress };
            }
            _ => {}
        }
    }

    /// 状态转换：Downloading → Paused
    pub fn pause(&mut self) {
        if let DownloadState::Downloading { progress } = self.state {
            self.state = DownloadState::Paused { progress };
        }
    }

    /// 状态转换：→ Completed
    pub fn complete(&mut self) {
        self.state = DownloadState::Completed;
    }

    /// 状态转换：Downloading → Failed（重试次数未超限时回到 Pending）
    pub fn fail(&mut self, error: String) {
        if let DownloadState::Downloading { .. } = self.state {
            self.retries += 1;

            if self.retries >= self.max_retries {
                self.state = DownloadState::Failed { error };
            } else {
                self.state = DownloadState::Pending;
            }
        }
    }

    /// 获取已下载的字节数（如果临时文件已存在）
    fn get_existing_progress(&self) -> u64 {
        std::fs::metadata(&self.temp_file)
            .map(|m| m.len())
            .unwrap_or(0)
    }
}