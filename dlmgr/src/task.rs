use crate::error::DlMgrCompletionError;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::oneshot;

pub struct DownloadTask {
    pub(crate) content_length: u64,
    pub(crate) task_stats: Arc<TaskStats>,
    pub(crate) completion_handle: oneshot::Receiver<Result<(), DlMgrCompletionError>>,
}

impl DownloadTask {
    pub async fn await_completion(self) -> Result<(), DlMgrCompletionError> {
        self.completion_handle
            .await
            .unwrap_or_else(|_| Err(DlMgrCompletionError::CompletionHandleDropped))
    }

    pub fn content_length(&self) -> u64 {
        self.content_length
    }

    pub fn progress_provider(&self) -> ProgressProvider {
        ProgressProvider {
            content_length: self.content_length,
            task_stats: self.task_stats.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProgressProvider {
    pub(crate) content_length: u64,
    pub(crate) task_stats: Arc<TaskStats>,
}

impl ProgressProvider {
    pub fn bytes_downloaded(&self) -> u64 {
        self.task_stats.bytes_downloaded.load(Ordering::SeqCst)
    }

    pub fn content_length(&self) -> u64 {
        self.content_length
    }
    pub fn progress_percent(&self) -> f32 {
        (self.bytes_downloaded() as f32 / self.content_length as f32) * 100.0
    }
}

#[derive(Default, Debug)]
pub(crate) struct TaskStats {
    pub(crate) bytes_downloaded: AtomicU64,
    pub(crate) cached_bytes: AtomicU64,
}
