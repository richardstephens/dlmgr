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
}
