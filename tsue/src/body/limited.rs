use bytes::Buf;
use http::StatusCode;
use http_body::Frame;
use std::fmt;

use crate::response::IntoResponse;

pub fn limit_frame<B: Buf>(
    frame: Frame<B>,
    remaining: &mut usize,
) -> Option<Result<Frame<B>, LengthLimitError>> {
    if let Some(data) = frame.data_ref() {
        if data.remaining() > *remaining {
            *remaining = 0;
            Some(Err(LengthLimitError))
        } else {
            *remaining -= data.remaining();
            Some(Ok(frame))
        }
    } else {
        Some(Ok(frame))
    }
}

pub fn limit_size_hint(mut hint: http_body::SizeHint, remaining: usize) -> http_body::SizeHint {
    let n = u64::try_from(remaining).unwrap_or(u64::MAX);
    if hint.lower() >= n {
        hint.set_exact(n)
    } else if let Some(max) = hint.upper() {
        hint.set_upper(n.min(max))
    } else {
        hint.set_upper(n)
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
