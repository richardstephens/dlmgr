use crate::worker::RequestChunkError;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum TaskBuilderError {
    #[error("Invalid parameter value: {0}")]
    InvalidParameterValue(&'static str),
}

#[derive(Error, Debug)]
pub enum DlMgrSetupError {
    #[error("HEAD request to url={0} failed: {0}")]
    HeadRequestFailed(Url, reqwest::Error),
    #[error("Server does not support range requests")]
    RangeRequestsUnsupported,
    #[error("NoContentLengthHeader")]
    NoContentLengthHeader,
    #[error("InconsistentContentLength")]
    InconsistentContentLength,
    #[error("InvalidMaxBufferSize")]
    InvalidMaxBufferSize,
    #[error("ReqwestClientBuildError: {0}")]
    ReqwestClientBuildError(reqwest::Error),
}

#[derive(Error, Debug)]
pub enum DlMgrCompletionError {
    #[error("Completion handle unexpectedly dropped. This is probably a bug.")]
    CompletionHandleDropped,
    #[error("ReqwestClientBuildError: {0}")]
    ReqwestClientBuildError(reqwest::Error),
    #[error("Download task failed: {0}")]
    TaskFailed(#[source] DownloadWorkerError),
    #[error("Download task panicked: {0}")]
    TaskPanicked(#[source] tokio::task::JoinError),
}

#[derive(Error, Debug)]
pub enum DownloadWorkerError {
    #[error("Worker {0} too many consecutive non-fatal failures, giving up. Last error: {1}")]
    TooManyConsecutiveFailures(u8, reqwest::Error),
    #[error("Worker {0} encountered fatal error: {1}")]
    Fatal(u8, #[source] RequestChunkError),
    #[error("Re-order chunks error: {0}")]
    ReorderChunks(#[source] ReorderChunkError),
}

#[derive(Error, Debug)]
pub enum ReorderChunkError {
    #[error("Chunk consumer failed to consume bytes: {0}")]
    ChunkConsumer(#[source] anyhow::Error),
    #[error("next_offset={0} ahead of furthest_offset={1}")]
    NextOffsetAheadOfFurthestOffset(u64, u64),
    #[error("Received chunk with offset={0} when we've already moved on. next_offset={1}")]
    ReceivedUnexpectedChunk(u64, u64),
    #[error("received duplicate chunk at offset {0}")]
    DuplicateChunk(u64),
    #[error("Unreachable condition reached")]
    UnreachableCondition,
    #[error("Found {0} leftover chunks after completion")]
    LeftoverChunks(usize),
}
