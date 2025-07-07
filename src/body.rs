//! Request and Response Body.
use bytes::{Buf, Bytes};
use std::{
    io,
    sync::{
        Arc,
        atomic::{
            AtomicU64,
            Ordering::{Relaxed, SeqCst},
        },
    },
    task::{Poll, ready},
};
use tcio::io::AsyncIoRead;
use tokio::net::TcpStream;

mod collect;

pub use collect::Collect;

// ===== BodyInner =====

#[derive(Debug)]
pub(crate) struct BodyInner {
    pub(crate) io: TcpStream,
    /// Body size limit
    ///
    /// always equal to or more than `self.bytes.len()`
    pub(crate) remaining: AtomicU64,
}

impl BodyInner {
    pub(crate) fn new(io: TcpStream, remaining: AtomicU64) -> Self {
        Self { io, remaining }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.remaining.load(Relaxed) as _
    }

    pub(crate) fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }

    pub(crate) fn poll_read(&self, buf: &mut [u8], cx: &mut std::task::Context) -> Poll<io::Result<usize>> {
        if !self.has_remaining() {
            return Poll::Ready(Err(io_err!(QuotaExceeded)));
        }

        match ready!(self.io.poll_read(buf, cx)) {
            Ok(read_u) => {
                let read = read_u.try_into().unwrap_or(u64::MAX);
                self.remaining.fetch_sub(read, SeqCst);
                Poll::Ready(Ok(read_u))
            },
            Err(err) => Poll::Ready(Err(err)),
        }
    }

    /// Try to read body from underlying io into the buffer, advancing the buffer cursor.
    #[inline]
    pub fn poll_read_buf<B>(
        &self,
        buf: &mut B,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<usize>>
    where
        B: bytes::BufMut + ?Sized,
    {
        tcio::io::poll_read_fn(|buf, cx| self.poll_read(buf, cx), buf, cx)
    }
}

// ===== Body =====

// TODO: streaming response body ?

/// HTTP Body.
#[derive(Debug)]
pub struct Body {
    inner: Option<Arc<BodyInner>>,
    bytes: Bytes,
}

impl Body {
    /// Create exact size buffered [`Body`].
    #[inline]
    pub fn new(bytes: impl Into<Bytes>) -> Self {
        Self {
            inner: None,
            bytes: bytes.into(),
        }
    }

    /// Create an empty [`Body`].
    #[inline]
    pub fn empty() -> Self {
        Self {
            inner: None,
            bytes: Bytes::new(),
        }
    }

    #[inline]
    pub(crate) fn from_io(state: Arc<BodyInner>, bytes: Bytes) -> Self {
        Self {
            inner: Some(state),
            bytes,
        }
    }

    pub(crate) fn exact_len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns the remaining body length.
    #[inline]
    pub fn remaining(&self) -> usize {
        match self.inner.as_ref() {
            Some(ok) => ok.remaining(),
            None => 0,
        }
    }

    /// Returns `true` if there is more body to read.
    #[inline]
    pub fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }

    /// Try to read body from underlying io into the buffer.
    pub fn poll_read(&mut self, buf: &mut [u8], cx: &mut std::task::Context) -> Poll<io::Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0))
        }

        let mut remaining = self.remaining();

        if remaining == 0 {
            return Poll::Ready(Err(io_err!(QuotaExceeded)));
        }

        if self.bytes.has_remaining() {
            let read = buf.len().min(self.bytes.remaining()).min(remaining);

            buf[..read].copy_from_slice(&self.bytes[..read]);

            self.bytes.advance(read);

            if let Some(state) = self.inner.as_ref() {
                state.remaining.fetch_sub(read.try_into().unwrap_or(u64::MAX), SeqCst);
            }

            remaining -= read;

            // destination buf is already full,
            // or body limit exhausted
            if buf.len() == read || remaining == 0 {
                return Poll::Ready(Ok(read))
            }

            // or the destination is larger than partially read bytes
            debug_assert!(self.bytes.is_empty());
        }

        // io call, if it exists
        match self.inner.as_ref() {
            Some(ok) => ok.poll_read(buf, cx),
            None => Poll::Ready(Err(io_err!(QuotaExceeded))),
        }
    }

    /// Try to read body from underlying io into the buffer, advancing the buffer cursor.
    #[inline]
    pub fn poll_read_buf<B>(
        &mut self,
        buf: &mut B,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<usize>>
    where
        B: bytes::BufMut + ?Sized,
    {
        tcio::io::poll_read_fn(|buf, cx| self.poll_read(buf, cx), buf, cx)
    }

    /// only used for body writing in `rt`
    pub(crate) fn bytes_mut(&mut self) -> &mut Bytes {
        &mut self.bytes
    }

    /// Collect the entire body into [`BytesMut`][bytes::BytesMut].
    #[inline]
    pub fn collect(self) -> Collect {
        Collect::new(self)
    }

}

impl Default for Body {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

// ===== Macros =====

/// `io_err!(ConnectionAborted)`
/// `io_err!(ConnectionAborted, "already closed")`
/// `io_err!("already closed")`
macro_rules! io_err {
    ($kind:ident) => {
        io::Error::from(io::ErrorKind::$kind)
    };
    ($kind:ident,$e:expr) => {
        io::Error::new(io::ErrorKind::$kind, $e)
    };
    ($e:literal) => {
        io::Error::new(io::ErrorKind::InvalidData, $e)
    };
}

use {io_err};
