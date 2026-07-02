use crate::api::sequential_chunk_consumer::SequentialChunkConsumer;
use crate::error::{DlMgrCompletionError, DownloadWorkerError, RequestChunkError};
use crate::task::TaskStats;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::oneshot;
use tracing::debug;
use url::Url;

pub(crate) fn spawn_sequential_download(
    url: Url,
    client: reqwest::Client,
    chunk_consumer: Box<dyn SequentialChunkConsumer>,
    task_stats: Arc<TaskStats>,
    completion_tx: oneshot::Sender<Result<(), DlMgrCompletionError>>,
) {
    tokio::spawn(async move {
        let mut chunk_consumer = chunk_consumer;
        let result = match stream_body(&url, &client, &mut chunk_consumer, &task_stats).await {
            Ok(()) => {
                chunk_consumer.finalise().await;
                Ok(())
            }
            Err(e) => {
                chunk_consumer.on_failure().await;
                Err(DlMgrCompletionError::TaskFailed(
                    DownloadWorkerError::Fatal(0, e),
                ))
            }
        };
        completion_tx.send(result).ok();
    });
}

async fn stream_body(
    url: &Url,
    client: &reqwest::Client,
    chunk_consumer: &mut Box<dyn SequentialChunkConsumer>,
    task_stats: &TaskStats,
) -> Result<(), RequestChunkError> {
    debug!("Beginning sequential download of {url}");
    let resp = client
        .get(url.clone())
        .send()
        .await
        .map_err(RequestChunkError::Reqwest)?;

    let mut resp = resp
        .error_for_status()
        .map_err(RequestChunkError::Reqwest)?;

    while let Some(chunk) = resp.chunk().await.map_err(RequestChunkError::Reqwest)? {
        let chunk_len = chunk.len() as u64;
        if chunk_len == 0 {
            continue;
        }
        task_stats
            .bytes_downloaded
            .fetch_add(chunk_len, Ordering::SeqCst);
        chunk_consumer
            .consume_bytes(chunk.into())
            .await
            .map_err(|e| RequestChunkError::SubmitChunkError(e.context("sequential_downloader")))?;
    }

    Ok(())
}
