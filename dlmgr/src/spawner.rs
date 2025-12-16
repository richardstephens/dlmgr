use crate::api::sequential_chunk_consumer::SequentialChunkConsumer;
use crate::chunk_order::reorder_chunks;
use crate::error::{DlMgrCompletionError, DlMgrSetupError};
use crate::response_helpers::{assert_supports_range_requests, extract_content_length};
use crate::task::{DownloadTask, TaskStats};
use crate::task_builder::DownloadProps;
use crate::task_provider::TaskProvider;
use crate::urlset::UrlSet;
use crate::worker::{WorkerContext, download_worker};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinSet;
use tracing::{debug, error};
use url::Url;

struct TaskProps {
    #[allow(unused)]
    content_length: u64,
    task_stats: Arc<TaskStats>,
    dl_props: DownloadProps,
}
struct ValidateUrlOutcome {
    content_length: u64,
}

pub async fn spawn_download_task(
    url_set: UrlSet,
    chunk_consumer: Box<dyn SequentialChunkConsumer>,
    props: DownloadProps,
) -> Result<DownloadTask, DlMgrSetupError> {

    let mut content_length: Option<u64> = None;
    let all_urls = url_set.all();
    debug!("Validating from {} urls", all_urls.len());
    let mut validate_url_joinset = JoinSet::new();
    for url in all_urls {
        let client = props
            .client_provider
            .client()
            .map_err(DlMgrSetupError::ReqwestClientBuildError)?;
        validate_url_joinset.spawn(validate_url_retried(client, url.clone(), props.validate_retry_limit));

    }

    let content_length: u64 = content_length.ok_or(DlMgrSetupError::NoContentLengthHeader)?;
    debug!("All urls agreed that content_length={content_length}");

    let (chtx, chrx) = oneshot::channel();

    let task_stats = Arc::new(TaskStats::default());
    let download_task = DownloadTask {
        content_length,
        task_stats: task_stats.clone(),
        completion_handle: chrx,
    };

    let task_provider = TaskProvider::new_provider(&props, content_length)?;

    let task_props = TaskProps {
        content_length,
        task_stats,
        dl_props: props,
    };

    tokio::spawn(async move {
        let dl_result = exec_download(task_provider, url_set, task_props, chunk_consumer).await;
        chtx.send(dl_result).ok();
    });

    Ok(download_task)
}
async fn validate_url_retried(client: reqwest::Client, url: Url, retry_limit: u8)-> Result<ValidateUrlOutcome, DlMgrSetupError> {
    let mut tries = 0;
    loop {
        match validate_url(&client, &url).await {
            Ok(outcome) => return Ok(outcome),
            Err(e) => {
                tries += 1;
                if tries >= retry_limit {
                    return Err(e);
                } else {
                    tokio::time::sleep(Duration::from_millis(500 * tries as u64)).await;
                }
            }
        }

    }
}

async fn validate_url(client: &reqwest::Client, url: &Url) -> Result<ValidateUrlOutcome, DlMgrSetupError> {
    let head_resp = client
        .head(url.clone())
        .send()
        .await
        .map_err(|e| DlMgrSetupError::HeadRequestFailed(url, e))?;

    let content_length = extract_content_length(&head_resp)?;

    if assert_supports_range_requests(&head_resp).is_err() {
        attempt_one_byte_range_request(&client, url).await?;
    }

    Ok(ValidateUrlOutcome { content_length })

}

async fn attempt_one_byte_range_request(client: &reqwest::Client, url: &Url) -> Result<(), DlMgrSetupError> {
    
}


async fn exec_download(
    task_provider: TaskProvider,
    url_set: UrlSet,
    mut props: TaskProps,
    chunk_consumer: Box<dyn SequentialChunkConsumer>,
) -> Result<(), DlMgrCompletionError> {
    let (chunk_tx, chunk_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut join_set = JoinSet::new();

    //spawn workers
    for ii in 0..props.dl_props.task_count {
        let client  = props.dl_props.client_provider.client()
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
        props.task_stats.clone(),
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
