use bytes::{Buf, Bytes};
use http::StatusCode;
use http_body::Frame;
use hyper::body::Incoming;
use std::{
    pin::Pin,
    task::{ready, Context, Poll},
};

use crate::response::{IntoResponse, Response};

#[derive(Debug)]
pub struct Body {
    body: Incoming,
    remaining: usize,
}

impl Body {
    pub(crate) fn new(body: Incoming) -> Self {
        Self { body, remaining: 2_000_000 }
    }
}

impl http_body::Body for Body {
    type Data = Bytes;

    type Error = BodyError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let ok = ready!(Pin::new(&mut self.body).poll_frame(cx)?);
        match ok {
            Some(frame) => {
                let result = if let Some(data) = frame.data_ref() {
                    if data.remaining() > self.remaining {
                        self.remaining = 0;
                        Some(Err(BodyError::Limited))
                    } else {
                        self.remaining -= data.remaining();
                        Some(Ok(frame))
                    }
                } else {
                    Some(Ok(frame))
                };

                Poll::Ready(result)
            },
            None => Poll::Ready(None)
        }
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        use std::convert::TryFrom;
        let n = u64::try_from(self.remaining).unwrap_or(u64::MAX);
        let mut hint = self.body.size_hint();
        if hint.lower() >= n {
            hint.set_exact(n)
        } else if let Some(max) = hint.upper() {
            hint.set_upper(n.min(max))
        } else {
            hint.set_upper(n)
        }
        hint
    }
}

#[derive(Debug)]
pub enum BodyError {
    Hyper(hyper::Error),
    Limited,
}

impl From<hyper::Error> for BodyError {
    fn from(v: hyper::Error) -> Self {
        Self::Hyper(v)
    }
}

impl std::fmt::Display for BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyError::Hyper(error) => error.fmt(f),
            BodyError::Limited => f.write_str("payload too large"),
        }
    }
}

impl IntoResponse for BodyError {
    fn into_response(self) -> Response {
        match self {
            BodyError::Hyper(error) => error.into_response(),
            BodyError::Limited => (StatusCode::PAYLOAD_TOO_LARGE, "payload too large").into_response(),
        }
    }
}

