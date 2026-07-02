use crate::error::DlMgrSetupError;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, HeaderValue};

pub(crate) fn detect_range_support(resp: &reqwest::Response) -> bool {
    match resp.headers().get(ACCEPT_RANGES) {
        Some(val) if val != HeaderValue::from_static("none") => true,
        _ => false,
    }
}

pub(crate) fn extract_content_length(resp: &reqwest::Response) -> Result<u64, DlMgrSetupError> {
    resp.headers()
        .get(CONTENT_LENGTH)
        .ok_or(DlMgrSetupError::NoContentLengthHeader)?
        .to_str()
        .map_err(|_| DlMgrSetupError::NoContentLengthHeader)
        .and_then(|v| {
            v.parse::<u64>()
                .map_err(|_| DlMgrSetupError::NoContentLengthHeader)
        })
}
