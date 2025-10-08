use crate::error::DlMgrCompletionError;
use tokio::sync::oneshot;

pub struct DownloadTask {
    pub(crate) content_length: u64,
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
}
