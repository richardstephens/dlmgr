use crate::error::DlMgrCompletionError;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::oneshot;

pub struct DownloadTask {
    pub(crate) content_length: u64,
    pub(crate) bytes_downloaded: Arc<AtomicU64>,
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
            bytes_downloaded: self.bytes_downloaded.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ProgressProvider {
    pub(crate) content_length: u64,
    pub(crate) bytes_downloaded: Arc<AtomicU64>,
}
impl ProgressProvider {
    pub fn bytes_downloaded(&self) -> u64 {
        self.bytes_downloaded.load(Ordering::SeqCst)
    }

    pub fn content_length(&self) -> u64 {
        self.content_length
    }
    pub fn progress_percent(&self) -> f32 {
        (self.bytes_downloaded() as f32 / self.content_length as f32) * 100.0
    }
}
