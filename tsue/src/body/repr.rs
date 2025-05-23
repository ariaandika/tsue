use bytes::Bytes;
use http_body::{Frame, SizeHint};
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll, ready},
};

use crate::response::{IntoResponse, Response};

/// Agnostic implementation of [`http_body::Body`].
pub enum Repr {
    Incoming(hyper::body::Incoming),
    Full(Bytes),
    Empty,
}

impl From<hyper::body::Incoming> for Repr {
    fn from(value: hyper::body::Incoming) -> Self {
        Self::Incoming(value)
    }
}

impl http_body::Body for Repr {
    type Data = Bytes;

    type Error = ReprBodyError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let ok = match self.get_mut() {
            Repr::Incoming(incoming) => ready!(Pin::new(incoming).poll_frame(cx)?),
            Repr::Full(bytes) if bytes.is_empty() => None,
            Repr::Full(bytes) => Some(Frame::data(std::mem::take(bytes))),
            Repr::Empty => None,
        };

        Poll::Ready(ok.map(Ok))
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Repr::Incoming(incoming) => incoming.is_end_stream(),
            Repr::Full(bytes) => bytes.is_empty(),
            Repr::Empty => true,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Repr::Incoming(incoming) => incoming.size_hint(),
            Repr::Full(bytes) => SizeHint::with_exact(bytes.len().try_into().unwrap_or(u64::MAX)),
            Repr::Empty => SizeHint::new(),
        }
    }
}

impl Default for Repr {
    fn default() -> Self {
        Self::Empty
    }
}

// ===== Error =====

pub enum ReprBodyError {
    Incoming(hyper::Error),
}

impl From<hyper::Error> for ReprBodyError {
    fn from(v: hyper::Error) -> Self {
        Self::Incoming(v)
    }
}

impl std::error::Error for ReprBodyError {}

impl fmt::Display for ReprBodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReprBodyError::Incoming(error) => error.fmt(f),
        }
    }
}

impl fmt::Debug for ReprBodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReprBodyError::Incoming(error) => error.fmt(f),
        }
    }
}

impl IntoResponse for ReprBodyError {
    fn into_response(self) -> Response {
        match self {
            ReprBodyError::Incoming(error) => error.into_response(),
        }
    }
}
