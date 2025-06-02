use bytes::Bytes;
use http_body::Frame;
use std::{
    fmt, pin::Pin, task::{ready, Context, Poll}
};

mod repr;
mod limited;
mod collect;
mod error;

use repr::Repr;

pub use limited::LengthLimitError;
pub use collect::{Collect, Collected};
pub use error::{BodyError, Kind};

// ===== Body =====

/// HTTP Body.
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

    /// Buffer the entire body into memory.
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

impl Default for Body {
    fn default() -> Self {
        Self::new(Repr::Empty)
    }
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Body").finish()
    }
}

