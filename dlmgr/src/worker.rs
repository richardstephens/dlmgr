use crate::urlset::UrlSet;
use anyhow::bail;
use async_channel::{Receiver, RecvError};

use reqwest::header::RANGE;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;
use tracing::{trace, warn};
use url::Url;

#[derive(Error, Debug)]
pub enum RequestChunkError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("SubmitChunkError")]
    SubmitChunkError,
}

pub(crate) struct WorkerContext {
    pub(crate) worker_num: u8,
    pub(crate) rx: Receiver<DlWorkerTask>,
    pub(crate) url_set: UrlSet,
    pub(crate) client: reqwest::Client,
    pub(crate) tx: ChunkSender,
}

#[derive(Debug)]
pub(crate) struct DlWorkerTask {
    pub offset: u64,
    pub len: u64,
}

type ChunkSender = UnboundedSender<(u64, Vec<u8>)>;

pub async fn download_worker(ctx: WorkerContext) -> anyhow::Result<()> {
    trace!("Beginning worker task {}", ctx.worker_num);
    loop {
        match ctx.rx.recv().await {
            Ok(wtask) => {
                retry_request_chunk(&ctx, &wtask).await?;
            }
            Err(RecvError) => {
                trace!("Worker {} exiting", ctx.worker_num);
                return Ok(());
            }
        }
    }
}
async fn retry_request_chunk(ctx: &WorkerContext, wtask: &DlWorkerTask) -> anyhow::Result<()> {
    let mut offset = wtask.offset;
    let mut len = wtask.len;
    let mut backoff = Duration::from_secs(1);
    let mut last_success = Instant::now();
    let mut attempt = 0;
    loop {
        // currently we rely on reqwest's timeout. Should we add our own timeout here?
        let url = ctx.url_set.url(ctx.worker_num, attempt);
        attempt += 1;
        match request_chunk(ctx, url.clone(), offset, len).await {
            Ok(received_len) => {
                attempt = 0;
                backoff = Duration::from_secs(1);
                last_success = Instant::now();
                if received_len > len {
                    bail!(
                        "Worker {} received excess bytes from url={url}. wtask={wtask:?} len={len} received_len={received_len}",
                        ctx.worker_num
                    );
                } else if received_len == len {
                    return Ok(());
                } else {
                    offset += received_len;
                    len -= received_len;
                }
            }
            Err(RequestChunkError::SubmitChunkError) => {
                bail!("Failed to submit chunk");
            }
            Err(RequestChunkError::Reqwest(e)) => {
                if last_success.elapsed() > Duration::from_secs(60 * 30) {
                    bail!(
                        "Worker {} too many consecutive failures, giving up. Last error: {:?}",
                        ctx.worker_num,
                        e
                    );
                } else {
                    warn!("Worker {} Error downloading chunk: {:?}", ctx.worker_num, e);
                }
                if backoff.as_secs() < 60 {
                    backoff = backoff * 2;
                }
            }
        }
    }
}

async fn request_chunk(
    ctx: &WorkerContext,
    url: Url,
    request_offset: u64,
    len: u64,
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
                ctx.tx
                    .send((request_offset + bytes_sent, chunk.into()))
                    .map_err(|_| RequestChunkError::SubmitChunkError)?;

                bytes_sent += chunk_len;
            }
            Ok(None) => {
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
