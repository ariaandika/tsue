use bytes::Bytes;
use http_body::Frame;
use std::{
    pin::Pin,
    task::{ready, Context, Poll},
};

use crate::response::{IntoResponse, Response};

mod repr;
mod limited;
mod collect;

pub(crate) use repr::Repr;
pub use collect::{Collect, Collected};

use repr::ReprBodyError;
use limited::LengthLimitError;

// ===== Body =====

pub struct Body {
    repr: Repr,
    remaining: Option<u64>,
}

impl<B: Into<Repr>> From<B> for Body {
    fn from(value: B) -> Self {
        Self::new(value)
    }
}

impl Body {
    pub(crate) fn new(repr: impl Into<Repr>) -> Self {
        Self { repr: repr.into(), remaining: Some(2_000_000) }
    }

    pub fn collect_body(self) -> Collect {
        Collect::new(self)
    }
}

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Some(ok) => ok,
            None => return Poll::Ready(None),
        }
    };
}

impl http_body::Body for Body {
    type Data = Bytes;

    type Error = BodyError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let frame = tri!(ready!(Pin::new(&mut self.repr).poll_frame(cx)?));
        let frame_result = match self.remaining.as_mut() {
            Some(remaining) => tri!(limited::limit_frame(frame, remaining)),
            None => Ok(frame),
        };
        Poll::Ready(Some(frame_result.map_err(Into::into)))
    }

    fn is_end_stream(&self) -> bool {
        self.repr.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        match self.remaining {
            Some(remaining) => limited::limit_size_hint(self.repr.size_hint(), remaining),
            None => self.repr.size_hint(),
        }
    }
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Body").finish()
    }
}

impl Default for Body {
    fn default() -> Self {
        Self { repr: Repr::Empty, remaining: None }
    }
}

// ===== Error =====

pub struct BodyError {
    kind: Kind,
}

impl BodyError {
    fn new(kind: Kind) -> Self {
        Self { kind }
    }
}

enum Kind {
    Incoming(hyper::Error),
    Limited(LengthLimitError),
}

impl IntoResponse for BodyError {
    fn into_response(self) -> Response {
        match self.kind {
            Kind::Incoming(r) => r.into_response(),
            Kind::Limited(l) => l.into_response(),
        }
    }
}

impl From<ReprBodyError> for BodyError {
    fn from(v: ReprBodyError) -> Self {
        match v {
            ReprBodyError::Incoming(error) => Self::new(Kind::Incoming(error)),
        }
    }
}

impl From<LengthLimitError> for BodyError {
    fn from(v: LengthLimitError) -> Self {
        Self::new(Kind::Limited(v))
    }
}

impl std::error::Error for BodyError { }

impl std::fmt::Debug for BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_tuple("BodyError");
        match &self.kind {
            Kind::Incoming(r) => f.field(&r),
            Kind::Limited(l) => f.field(&l),
        }.finish()
    }
}

impl std::fmt::Display for BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            Kind::Incoming(r) => r.fmt(f),
            Kind::Limited(l) => l.fmt(f),
        }
    }
}

