//! Request and Response Body.
use bytes::{Bytes, BytesMut};
use std::{io, task::Poll};

mod handle;
mod stream;
mod collect;

use handle::{BodyHandle, IoHandle};
use stream::BodyStreamHandle;

pub use stream::BodyStream;
pub use collect::Collect;


#[derive(Debug, Default)]
pub struct Body {
    repr: Repr,
}

#[derive(Debug, Default)]
enum Repr {
    #[default]
    Empty,
    Bytes(Bytes),
    Handle(BodyHandle),
    Stream(BodyStreamHandle)
}

// ===== Constructor =====

impl Body {
    /// Create an exact size [`Body`].
    #[inline]
    pub fn new(bytes: impl Into<Bytes>) -> Body {
        Self {
            repr: Repr::Bytes(bytes.into()),
        }
    }

    /// Create an empty [`Body`].
    #[inline]
    pub const fn empty() -> Body {
        Self { repr: Repr::Empty }
    }

    #[inline]
    #[allow(unused, reason = "todo")]
    pub(crate) fn from_handle(handle: IoHandle, remaining: u64, remain: BytesMut) -> Self {
        Self {
            repr: Repr::Handle(BodyHandle::new(handle, remaining, remain)),
        }
    }

    /// Create a [`Body`] with given [`BodyStream`].
    #[inline]
    pub fn stream<S>(stream: S) -> Body
    where
        S: BodyStream,
    {
        Self {
            repr: Repr::Stream(BodyStreamHandle::new(stream)),
        }
    }
}

// ===== Ref =====

impl Body {
    #[inline]
    pub fn remaining(&self) -> usize {
        match &self.repr {
            Repr::Empty => 0,
            Repr::Bytes(bytes) => bytes.len(),
            Repr::Handle(handle) => handle.remaining(),
            Repr::Stream(stream) => stream.remaining(),
        }
    }

    #[inline]
    pub fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }
}

// ===== Read =====

impl Body {
    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<Bytes>> {
        match &mut self.repr {
            Repr::Empty => Poll::Ready(Err(io::ErrorKind::QuotaExceeded.into())),
            Repr::Bytes(b) => Poll::Ready(if b.is_empty() {
                Err(io::ErrorKind::QuotaExceeded.into())
            } else {
                Ok(std::mem::take(b))
            }),
            Repr::Handle(handle) => handle.poll_read(cx).map_ok(BytesMut::freeze),
            Repr::Stream(stream) => stream.poll_read(cx),
        }
    }

    #[inline]
    pub fn read(&mut self) -> impl Future<Output = io::Result<Bytes>> {
        std::future::poll_fn(|cx| self.poll_read(cx))
    }

    #[inline]
    pub fn collect(self) -> Collect {
        Collect::new(self)
    }
}
