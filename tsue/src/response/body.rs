use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use http_body::Frame;
use http_body_util::Full;

/// HTTP Response Body.
#[derive(Debug)]
pub enum Body {
    Full(Full<Bytes>),
}

impl Default for Body {
    fn default() -> Self {
        Self::Full(Bytes::new().into())
    }
}

macro_rules! from {
    (|$fr:ty:$pat:pat_param|$body:expr) => {
        impl From<$fr> for Body {
            fn from($pat: $fr) -> Self {
                $body
            }
        }
    };
}

from!(|Bytes:b|Self::Full(b.into()));
from!(|BytesMut:b|Self::Full(b.freeze().into()));
from!(|Vec<u8>:b|Self::Full(b.into()));

impl http_body::Body for Body {
    type Data = Bytes;

    type Error = hyper::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            Body::Full(full) => Pin::new(full).poll_frame(cx).map_err(|e| match e {}),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Body::Full(full) => full.is_end_stream(),
        }
    }

    fn size_hint(&self) -> http_body::SizeHint {
        match self {
            Body::Full(full) => full.size_hint(),
        }
    }
}
