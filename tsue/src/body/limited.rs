use bytes::Buf;
use http::StatusCode;
use http_body::Frame;
use std::fmt;

use crate::response::IntoResponse;

pub fn limit_frame<B: Buf>(
    frame: Frame<B>,
    remaining: &mut u64,
) -> Option<Result<Frame<B>, LengthLimitError>> {
    if let Some(data) = frame.data_ref() {
        let data_remain = u64::try_from(data.remaining()).unwrap_or(u64::MAX);
        if data_remain as u64 > *remaining {
            *remaining = 0;
            Some(Err(LengthLimitError))
        } else {
            *remaining -= data_remain;
            Some(Ok(frame))
        }
    } else {
        Some(Ok(frame))
    }
}

pub fn limit_size_hint(mut hint: http_body::SizeHint, remaining: u64) -> http_body::SizeHint {
    if hint.lower() >= remaining {
        hint.set_exact(remaining)
    } else if let Some(max) = hint.upper() {
        hint.set_upper(remaining.min(max))
    } else {
        hint.set_upper(remaining)
    }
    hint
}

#[derive(Debug)]
pub struct LengthLimitError;

impl fmt::Display for LengthLimitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("length limit exceeded")
    }
}

impl IntoResponse for LengthLimitError {
    fn into_response(self) -> crate::response::Response {
        (StatusCode::PAYLOAD_TOO_LARGE, "payload too large").into_response()
    }
}
