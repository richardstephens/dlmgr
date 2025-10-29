use crate::api::sequential_chunk_consumer::SequentialChunkConsumer;
use crate::chunk_order::reorder_chunks;
use crate::error::{DlMgrCompletionError, DlMgrSetupError};
use crate::response_helpers::{assert_supports_range_requests, extract_content_length};
use crate::task::DownloadTask;
use crate::task_builder::DownloadProps;
use crate::task_provider::TaskProvider;
use crate::urlset::UrlSet;
use crate::worker::{WorkerContext, download_worker};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::oneshot;
use tokio::task::JoinSet;
use tracing::{debug, error};

struct TaskProps {
    content_length: u64,
    bytes_downloaded: Arc<AtomicU64>,
    initial_client: Option<reqwest::Client>,
    dl_props: DownloadProps,
}

pub async fn spawn_download_task(
    url_set: UrlSet,
    chunk_consumer: Box<dyn SequentialChunkConsumer>,
    props: DownloadProps,
) -> Result<DownloadTask, DlMgrSetupError> {
    let client = props
        .client_provider
        .client()
        .map_err(DlMgrSetupError::ReqwestClientBuildError)?;
    let mut content_length: Option<u64> = None;
    let all_urls = url_set.all();
    debug!("Validating from {} urls", all_urls.len());
    for url in all_urls {
        let head_resp = client
            .head(url.clone())
            .send()
            .await
            .map_err(|e| DlMgrSetupError::HeadRequestFailed(url, e))?;

        assert_supports_range_requests(&head_resp)?;

        let this_content_length = extract_content_length(&head_resp)?;
        if let Some(cl) = content_length {
            if cl != this_content_length {
                return Err(DlMgrSetupError::InconsistentContentLength);
            }
        } else {
            content_length = Some(this_content_length);
        }
    }

    let content_length: u64 = content_length.ok_or(DlMgrSetupError::NoContentLengthHeader)?;
    debug!("All urls agreed that content_length={content_length}");

    let (chtx, chrx) = oneshot::channel();

    let bytes_downloaded = Arc::new(AtomicU64::new(0));
    let download_task = DownloadTask {
        content_length,
        bytes_downloaded: bytes_downloaded.clone(),
        completion_handle: chrx,
    };

    let task_props = TaskProps {
        content_length,
        bytes_downloaded,
        initial_client: Some(client),
        dl_props: props,
    };

    tokio::spawn(async move {
        let dl_result = exec_download(url_set, task_props, chunk_consumer).await;
        chtx.send(dl_result).ok();
    });

    Ok(download_task)
}

async fn exec_download(
    url_set: UrlSet,
    mut props: TaskProps,
    chunk_consumer: Box<dyn SequentialChunkConsumer>,
) -> Result<(), DlMgrCompletionError> {
    let task_provider = TaskProvider::new_provider(props.dl_props.chunk_size, props.content_length);

    let (chunk_tx, chunk_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut join_set = JoinSet::new();

    //spawn workers
    for ii in 0..props.dl_props.task_count {
        let client = props
            .initial_client
            .take()
            .map(Ok)
            .unwrap_or_else(|| props.dl_props.client_provider.client())
            .map_err(|e| DlMgrCompletionError::ReqwestClientBuildError(e))?;
        join_set.spawn(download_worker(WorkerContext {
            worker_num: ii,
            task_provider: task_provider.clone(),
            url_set: url_set.clone(),
            client,
            tx: chunk_tx.clone(),
        }));
    }

    // This drop is load-bearing - the `reorder_chunks` fn relies on the channels closing
    // to be able to know that there are no more messages to process.
    drop(chunk_tx);

    join_set.spawn(reorder_chunks(
        chunk_rx,
        chunk_consumer,
        props.bytes_downloaded.clone(),
    ));

    // all of the tasks in the set should complete and return Ok(()). if any of them fail to do so,
    // we should bail, effectively cancelling the download.
    while let Some(outcome) = join_set.join_next().await {
        match outcome {
            Ok(Ok(())) => {}
            x => error!("{:?}", x),
        }
    }

    Ok(())
}
