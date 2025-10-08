use crate::error::DlMgrSetupError;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, HeaderValue};

pub(crate) fn assert_supports_range_requests(
    resp: &reqwest::Response,
) -> Result<(), DlMgrSetupError> {
    let header_val = resp
        .headers()
        .get(ACCEPT_RANGES)
        .ok_or(DlMgrSetupError::RangeRequestsUnsupported)?;
    if header_val == HeaderValue::from_static("none") {
        Err(DlMgrSetupError::RangeRequestsUnsupported)
    } else {
        Ok(())
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
