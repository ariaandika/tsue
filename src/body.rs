//! Request and Response Body.
use tcio::bytes::{Bytes, BytesMut};
use std::{io, task::Poll};

mod handle;
mod stream;
mod collect;
mod writer;

// =====

use handle::{BodyHandle, IoHandle};
use stream::BodyStreamHandle;

pub use stream::BodyStream;
pub use collect::Collect;

pub(crate) use writer::BodyWrite;

/// HTTP Body
#[derive(Debug, Default)]
pub struct Body {
    repr: Repr,
}

/// [`Body`] can be standalone bytes or holds a handle to an IO stream.
#[derive(Debug)]
enum Repr {
    Bytes(Bytes),
    Handle(BodyHandle),
    Stream(BodyStreamHandle)
}

impl Default for Repr {
    #[inline]
    fn default() -> Self {
        Repr::Bytes(Bytes::new())
    }
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
        Self {
            repr: Repr::Bytes(Bytes::new()),
        }
    }

    #[inline]
    pub(crate) fn from_handle(handle: IoHandle, remaining: u64) -> Self {
        Self {
            repr: Repr::Handle(BodyHandle::new(handle, remaining)),
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

    #[inline]
    pub(crate) fn into_writer(self) -> BodyWrite {
        BodyWrite::new(self)
    }
}

// ===== Ref =====

impl Body {
    #[inline]
    pub fn remaining(&self) -> usize {
        match &self.repr {
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
    /// Tries to read data from the stream and returns the buffer.
    ///
    /// If body is exhausted, or no body contained, returns an error. Use [`Body::has_remaining`]
    /// to check whether there are bytes remaining to read.
    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<Bytes>> {
        match &mut self.repr {
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
