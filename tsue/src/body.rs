use bytes::Bytes;
use http_body::Frame;
use std::{
    pin::Pin,
    task::{ready, Context, Poll},
};

use crate::response::{IntoResponse, Response};

mod repr;
mod limited;

pub(crate) use repr::Repr;

use repr::ReprBodyError;
use limited::LengthLimitError;

// ===== Body =====

pub struct Body {
    repr: Repr,
    remaining: u64,
}

impl Body {
    pub(crate) fn new(repr: impl Into<Repr>) -> Self {
        Self::with_limit(repr, 2_000_000)
    }

    pub(crate) fn with_limit(repr: impl Into<Repr>, limit: u64) -> Self {
        Self { repr: repr.into(), remaining: limit }
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
        let ok = tri!(ready!(Pin::new(&mut self.repr).poll_frame(cx)?));
        let ok = tri!(limited::limit_frame(ok, &mut self.remaining));
        Poll::Ready(Some(ok.map_err(Into::into)))
    }

    fn is_end_stream(&self) -> bool {
        self.repr.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        limited::limit_size_hint(self.repr.size_hint(), self.remaining)
    }
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Body").finish()
    }
}

impl Default for Body {
    fn default() -> Self {
        Self { repr: Repr::Empty, remaining: 0 }
    }
}

// ===== Error =====

pub struct BodyError {
    kind: Kind,
}

enum Kind {
    Repr(ReprBodyError),
    Limited(LengthLimitError),
}

impl From<ReprBodyError> for BodyError {
    fn from(v: ReprBodyError) -> Self {
        Self {
            kind: Kind::Repr(v),
        }
    }
}

impl From<LengthLimitError> for BodyError {
    fn from(v: LengthLimitError) -> Self {
        Self {
            kind: Kind::Limited(v),
        }
    }
}

impl std::error::Error for BodyError { }

impl std::fmt::Debug for BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_tuple("BodyError");
        match &self.kind {
            Kind::Repr(r) => f.field(&r),
            Kind::Limited(l) => f.field(&l),
        }.finish()
    }
}

impl std::fmt::Display for BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            Kind::Repr(r) => r.fmt(f),
            Kind::Limited(l) => l.fmt(f),
        }
    }
}

impl IntoResponse for BodyError {
    fn into_response(self) -> Response {
        match self.kind {
            Kind::Repr(r) => r.into_response(),
            Kind::Limited(l) => l.into_response(),
        }
    }
}

