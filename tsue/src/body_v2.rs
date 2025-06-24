use std::{io, sync::Arc, task::{ready, Poll}};
use bytes::{Buf, Bytes};
use tcio::{io::AsyncIo, tokio::IoStream};

mod collect;

pub use collect::Collect;

/// HTTP Body.
#[derive(Debug)]
pub struct Body {
    #[cfg(feature = "tokio")]
    io: Option<Arc<IoStream>>,

    /// Body size limit
    ///
    /// always equal to or more than `self.bytes.len()`
    remaining: u64,

    /// Buffer
    bytes: Bytes,
}

impl Body {
    /// Create an empty [`Body`].
    pub fn empty() -> Self {
        Self { io: None, remaining: 0, bytes: Bytes::new()  }
    }

    pub fn exact(bytes: impl Into<Bytes>) -> Self {
        let bytes = bytes.into();
        Self { io: None, remaining: bytes.len().try_into().unwrap_or(u64::MAX), bytes }
    }

    #[cfg(feature = "tokio")]
    #[allow(unused, reason = "used later")]
    pub(crate) fn from_io(io: Arc<IoStream>, remaining: u64, bytes: Bytes) -> Self {
        Self { io: Some(io), remaining, bytes  }
    }

    pub fn remaining(&self) -> usize {
        self.remaining as _
    }

    pub fn has_remaining(&self) -> bool {
        self.remaining != 0
    }

    pub fn poll_read(&mut self, buf: &mut [u8], cx: &mut std::task::Context) -> Poll<io::Result<usize>> {
        debug_assert!(self.remaining as usize >= self.bytes.len());

        if self.remaining == 0 {
            return Poll::Ready(Err(io_err!(QuotaExceeded)));
        }

        if buf.is_empty() {
            return Poll::Ready(Ok(0))
        }

        if self.bytes.has_remaining() {
            let read = buf.len().min(self.bytes.remaining()).min(self.remaining as usize);

            buf[..read].copy_from_slice(&self.bytes[..read]);

            self.bytes.advance(read);
            self.remaining -= read as u64;

            // destination buf is already full,
            // or body limit exhausted
            if buf.len() == read || self.remaining == 0 {
                return Poll::Ready(Ok(read))
            }

            // or the destination is larger than partially read bytes
            debug_assert!(self.bytes.is_empty());
        }

        // io call, if it exists
        let Some(io) = self.io.as_mut() else {
            return Poll::Ready(Err(io_err!(QuotaExceeded)));
        };

        let result = ready!(io.poll_read(buf, cx))
            .inspect(|&read|self.remaining -= read.try_into().unwrap_or(u64::MAX));

        Poll::Ready(result)
    }

    pub fn poll_read_buf<B>(
        &mut self,
        buf: &mut B,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<usize>>
    where
        B: bytes::BufMut + ?Sized,
    {
        if !buf.has_remaining_mut() {
            return Poll::Ready(Ok(0));
        }

        let read = {
            // SAFETY: we will only write initialized value and `MaybeUninit<T>` is guaranteed to
            // have the same size as `T`:
            let dst = unsafe {
                &mut *(buf.chunk_mut().as_uninit_slice_mut() as *mut [std::mem::MaybeUninit<u8>]
                    as *mut [u8])
            };

            tri!(ready!(self.poll_read(dst, cx)))
        };

        // SAFETY: This is guaranteed to be the number of initialized by `try_read`
        unsafe {
            buf.advance_mut(read);
        }

        Poll::Ready(Ok(read))
    }

    pub fn collect(self) -> Collect {
        Collect::new(self)
    }

}

impl Default for Body {
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

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(ok) => ok,
            Err(err) => return Poll::Ready(Err(err)),
        }
    };
}

use {io_err, tri};
