use bytes::Bytes;
use http_body::{Frame, SizeHint};
use hyper::body::Incoming;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::BodyError;

/// Agnostic implementation of [`http_body::Body`].
#[derive(Debug)]
pub enum Repr {
    Incoming(Incoming),
    Full(Bytes),
    Empty,
}

impl From<Incoming> for Repr {
    fn from(value: Incoming) -> Self {
        Self::Incoming(value)
    }
}

impl From<Vec<u8>> for Repr {
    fn from(value: Vec<u8>) -> Self {
        Self::Full(value.into())
    }
}

impl From<String> for Repr {
    fn from(value: String) -> Self {
        Self::Full(value.into_bytes().into())
    }
}

impl From<Bytes> for Repr {
    fn from(value: Bytes) -> Self {
        Self::Full(value)
    }
}

impl From<&'static str> for Repr {
    fn from(value: &'static str) -> Self {
        Self::Full(value.into())
    }
}

impl http_body::Body for Repr {
    type Data = Bytes;

    type Error = BodyError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let ok = match self.get_mut() {
            Repr::Incoming(incoming) => ready!(Pin::new(incoming).poll_frame(cx)?),
            Repr::Full(bytes) => {
                if bytes.is_empty() {
                    None
                } else {
                    Some(Frame::data(std::mem::take(bytes)))
                }
            }
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

