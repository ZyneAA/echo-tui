pub struct DownloadJob {
    pub id: u32,
    pub url: String,
    pub metadata: Metadata,
    pub status: JobStatus,
}

pub enum JobStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed(String),
}

pub struct DownloadQueue {
    pub jobs: Vec<DownloadJob>,
    pub max_running: usize,
    pub command_rx: UnboundedReceiver<QueueCommand>,
    pub report_tx: Sender<Report>,
}

pub enum QueueCommand {
    AddJob(DownloadJob),
    PauseJob(u32),
    ResumeJob(u32),
    CancelJob(u32),
    RemoveJob(u32),
    MoveUp(u32),
    MoveDown(u32),
}
