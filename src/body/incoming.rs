use std::task::Poll;
use tcio::bytes::Bytes;

use super::Collect;
use super::error::ReadError;
use super::handle::{BodyHandle, IoHandle};

/// [`Body`] implemenation for HTTP server.
///
/// [`Body`]: super::Body
#[derive(Debug, Default)]
pub struct Incoming {
    repr: Repr,
}

/// [`Body`] can be standalone bytes or holds a handle to an IO stream.
#[derive(Debug)]
enum Repr {
    Bytes(Bytes),
    Handle(BodyHandle),
}

impl Default for Repr {
    #[inline]
    fn default() -> Self {
        Repr::Bytes(Bytes::new())
    }
}

// ===== Constructor =====

impl Incoming {
    /// Create an exact size [`Body`].
    #[inline]
    pub fn new(bytes: impl Into<Bytes>) -> Incoming {
        Self {
            repr: Repr::Bytes(bytes.into()),
        }
    }

    /// Create an empty [`Body`].
    #[inline]
    pub const fn empty() -> Incoming {
        Self {
            repr: Repr::Bytes(Bytes::new()),
        }
    }

    #[inline]
    pub(crate) fn from_handle(handle: IoHandle, size_hint: Option<u64>) -> Self {
        Self {
            repr: Repr::Handle(BodyHandle::new(handle, size_hint)),
        }
    }

    // #[inline]
    // pub(crate) fn into_writer(self) -> BodyWrite {
    //     BodyWrite::new(self)
    // }
}

// ===== Ref =====

impl Incoming {
    /// Returns the bounds on the remaining length of the message body.
    ///
    /// Specifically, `size_hint()` returns a tuple where the first element is the lower bound, and
    /// the second element is the upper bound.
    ///
    /// The second half of the tuple that is returned is an [Option<usize>]. A [`None`] here means
    /// that either there is no known upper bound, or the upper bound is larger than [`usize`].
    pub fn size_hint(&self) -> (u64, Option<u64>) {
        match &self.repr {
            Repr::Bytes(b) => (b.len() as u64, Some(b.len() as u64)),
            Repr::Handle(handle) => (handle.size_hint().unwrap_or(0), handle.size_hint()),
        }
    }

    // pub(super) fn repr(&self) -> &Repr {
    //     &self.repr
    // }
}

// ===== Read =====

impl Incoming {
    #[inline]
    pub fn read(&mut self) -> impl Future<Output = Option<Result<Bytes, ReadError>>> {
        std::future::poll_fn(|cx| self.poll_read(cx))
    }

    #[inline]
    pub fn collect(self) -> Collect {
        Collect::new(self)
    }

    /// Tries to read data from the stream and returns the buffer.
    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<Option<Result<Bytes, ReadError>>> {
        match &mut self.repr {
            Repr::Bytes(b) => Poll::Ready(if b.is_empty() {
                None
            } else {
                Some(Ok(std::mem::take(b)))
            }),
            Repr::Handle(handle) => handle.poll_read(cx),
        }
    }
}

