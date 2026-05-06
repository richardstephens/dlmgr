use crate::urlset::UrlSet;

use crate::error::DownloadWorkerError;
use crate::task_provider::TaskProvider;
use reqwest::header::RANGE;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;
use tracing::{debug, trace, warn};
use url::Url;

#[derive(Error, Debug)]
pub enum RequestChunkError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("SubmitChunkError")]
    SubmitChunkError,
    #[error("SplitPermitError: {0}")]
    SplitPermitError(String),
    #[error(
        "Received excess bytes from url={url}. task.offset={wtask_offset} task.len={wtask_len} offset={offset} len={len} received_len={received_len}"
    )]
    ExcessBytes {
        url: Url,
        wtask_offset: u64,
        wtask_len: u64,
        offset: u64,
        len: u64,
        received_len: u64,
    },
}

pub(crate) struct WorkerContext {
    pub(crate) worker_num: u8,
    pub(crate) task_provider: TaskProvider,
    pub(crate) url_set: UrlSet,
    pub(crate) client: reqwest::Client,
    pub(crate) tx: ChunkSender,
}

#[derive(Debug)]
pub(crate) struct DlWorkerTask {
    pub offset: u64,
    pub len: u64,
}

type ChunkSender = UnboundedSender<(u64, Vec<u8>, OwnedSemaphorePermit)>;

pub async fn download_worker(ctx: WorkerContext) -> Result<(), DownloadWorkerError> {
    debug!("Beginning worker task {}", ctx.worker_num);
    loop {
        match ctx.task_provider.next_task_throttled().await {
            Ok(Some((wtask, permit))) => {
                retry_request_chunk(&ctx, &wtask, permit).await?;
            }
            Ok(None) => {
                debug!("Worker {} exiting", ctx.worker_num);
                return Ok(());
            }
            Err(_) => {}
        }
    }
}
async fn retry_request_chunk(
    ctx: &WorkerContext,
    wtask: &DlWorkerTask,
    permit: OwnedSemaphorePermit,
) -> Result<(), DownloadWorkerError> {
    let mut offset = wtask.offset;
    let mut len = wtask.len;
    let mut backoff = Duration::from_secs(1);
    let mut last_success = Instant::now();
    let mut attempt = 0;

    let mut permit_container = Some(permit);
    loop {
        // currently we rely on reqwest's timeout. Should we add our own timeout here?
        let url = ctx.url_set.url(ctx.worker_num, attempt);
        attempt += 1;
        match request_chunk(ctx, url.clone(), offset, len, &mut permit_container).await {
            Ok(received_len) => {
                attempt = 0;
                backoff = Duration::from_secs(1);
                last_success = Instant::now();
                if received_len > len {
                    return Err(DownloadWorkerError::Fatal(
                        ctx.worker_num,
                        RequestChunkError::ExcessBytes {
                            url,
                            wtask_offset: wtask.offset,
                            wtask_len: wtask.len,
                            offset,
                            len,
                            received_len,
                        },
                    ));
                } else if received_len == len {
                    return Ok(());
                } else {
                    offset += received_len;
                    len -= received_len;
                }
            }
            Err(RequestChunkError::Reqwest(e)) => {
                if last_success.elapsed() > Duration::from_secs(60 * 30) {
                    return Err(DownloadWorkerError::TooManyConsecutiveFailures(
                        ctx.worker_num,
                        e,
                    ));
                } else {
                    warn!("Worker {} Error downloading chunk: {:?}", ctx.worker_num, e);
                }
                if backoff.as_secs() < 60 {
                    backoff *= 2;
                }
            }
            Err(e) => {
                return Err(DownloadWorkerError::Fatal(ctx.worker_num, e));
            }
        }
    }
}

async fn request_chunk(
    ctx: &WorkerContext,
    url: Url,
    request_offset: u64,
    len: u64,
    permit_container: &mut Option<OwnedSemaphorePermit>,
) -> Result<u64, RequestChunkError> {
    let range_end_pos = request_offset + len - 1;
    let range_header_val = format!("bytes={request_offset}-{range_end_pos}");
    trace!("Requesting range={range_header_val}");
    let mut resp = ctx
        .client
        .get(url)
        .header(RANGE, &range_header_val)
        .send()
        .await?;

    let mut bytes_sent: u64 = 0;
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                let chunk_len = chunk.len() as u64;

                // it turns out that, for HTTP2, reqwest will send us a 0-length chunk at the end.
                // No need to forward this onwards.
                if chunk_len > 0 {
                    let permits = split_permits(permit_container, chunk_len)?;
                    ctx.tx
                        .send((request_offset + bytes_sent, chunk.into(), permits))
                        .map_err(|_| RequestChunkError::SubmitChunkError)?;
                }

                bytes_sent += chunk_len;
            }
            Ok(None) => {
                //todo:does this make sense?
                if permit_container.is_some() {
                    return Err(RequestChunkError::SubmitChunkError);
                }
                return Ok(bytes_sent);
            }
            Err(e) => {
                return if bytes_sent > 0 {
                    warn!(
                        "worker {} encountered error after bytes already received: {:?}",
                        ctx.worker_num, e
                    );
                    Ok(bytes_sent)
                } else {
                    Err(RequestChunkError::Reqwest(e))
                };
            }
        }
    }
}

fn split_permits(
    permit_container: &mut Option<OwnedSemaphorePermit>,
    count: u64,
) -> Result<OwnedSemaphorePermit, RequestChunkError> {
    if let Some(permits) = permit_container.as_mut() {
        if let Some(split_permits) = permits.split(count as usize) {
            if permits.num_permits() == 0 {
                *permit_container = None;
            }
            Ok(split_permits)
        } else {
            Err(RequestChunkError::SplitPermitError(format!(
                "Not enough permits available. Requested={count} avail={}",
                permits.num_permits()
            )))
        }
    } else {
        Err(RequestChunkError::SplitPermitError(
            "permit_container already empty".into(),
        ))
    }
}
