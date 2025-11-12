use std::{
    cmp, io,
    sync::Arc,
    task::{Poll, ready},
};
use tcio::{
    bytes::BytesMut,
    io::{AsyncIoRead, AsyncIoWrite},
};

use super::Shared;

pub struct IoBuffer<IO> {
    io: IO,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    remaining: u64,
    shared: Arc<Shared>,
}

impl<IO> IoBuffer<IO> {
    pub fn new(io: IO) -> Self {
        Self {
            io,
            read_buffer: BytesMut::with_capacity(512),
            write_buffer: BytesMut::with_capacity(512),
            remaining: 0,
            shared: Arc::new(Shared::new()),
        }
    }

    pub fn read_buffer_mut(&mut self) -> &mut BytesMut {
        &mut self.read_buffer
    }

    pub fn write_buffer_mut(&mut self) -> &mut BytesMut {
        &mut self.write_buffer
    }

    pub fn set_remaining(&mut self, remaining: u64) {
        self.remaining = remaining;
    }

    pub fn handle(&self) -> IoHandle {
        IoHandle {
            shared: self.shared.clone(),
        }
    }

    pub fn clear_reclaim(&mut self) {
        self.read_buffer.clear();

        // `reserve` will try to reclaim buffer, but if the underlying buffer is grow thus
        // reallocated, and the new allocated capacity is not at least 512, reclaiming does not
        // work, so another reallocation required
        //
        // also this allocation does not need to copy any data
        self.read_buffer.reserve(512);
    }
}

impl<IO> IoBuffer<IO>
where
    IO: AsyncIoRead,
{
    pub(crate) fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<usize>> {
        self.io.poll_read_buf(&mut self.read_buffer, cx)
    }

    fn poll_read_remaining(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<()>> {
        if self.remaining == 0 {
            self.shared
                .set_read_result(Err(io::ErrorKind::QuotaExceeded.into()))?;
            return Poll::Ready(Ok(()));
        }

        let result = match ready!(self.io.poll_read_buf(&mut self.read_buffer, cx)) {
            Ok(read) => {
                // we SHOULD NOT read past remaining
                let read = cmp::min(read as _, self.remaining);

                // SAFETY: read <= self.remaining
                self.remaining = unsafe { self.remaining.unchecked_sub(read) };

                Ok(self.read_buffer.split_to(read as _))
            }
            Err(err) => Err(err),
        };

        Poll::Ready(self.shared.set_read_result(result))
    }

    fn poll_collect(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<()>> {
        while self.remaining != 0 {
            ready!(self.poll_read_remaining(cx))?;
        }
        Poll::Ready(Ok(()))
    }

    /// Poll for IO read by [`IoHandle`].
    ///
    /// If data is available, waker will be called.
    pub(crate) fn poll_io_wants(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<()>> {
        use super::WantsFlag;

        let Some(wants) = self.shared.wants_flag().load_is_want() else {
            // `IoHandle` have not been called yet
            return Poll::Ready(Ok(()));
        };

        let poll = if WantsFlag::is_want_read(wants) {
            self.poll_read_remaining(cx)
        } else {
            assert!(WantsFlag::is_want_collect(wants));
            self.poll_collect(cx)
        };

        ready!(poll)?;
        self.shared.wants_flag().set_available();
        cx.waker().wake_by_ref();
        Poll::Ready(Ok(()))
    }

    pub(crate) fn poll_drain(
        &mut self,
        cx: &mut std::task::Context,
    ) -> Poll<Result<(), io::Error>> {
        // clear internal buffer
        // clear remaining pending body
        // clear already sent buffer

        self.read_buffer.clear();

        while self.remaining > 0 {
            let read = ready!(self.poll_read(cx))?;
            self.remaining = self.remaining.saturating_sub(read as _);
            self.read_buffer.clear();
        }

        self.shared.take_read_result()?;
        self.shared.wants_flag().set_idle();

        Poll::Ready(Ok(()))
    }
}

impl<IO> IoBuffer<IO>
where
    IO: AsyncIoWrite,
{
    pub(crate) fn poll_write(
        &mut self,
        buf: &[u8],
        cx: &mut std::task::Context,
    ) -> Poll<Result<usize, io::Error>> {
        self.io.poll_write(buf, cx)
    }

    pub(crate) fn poll_flush(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<()>> {
        if self.write_buffer.is_empty() {
            Poll::Ready(Ok(()))
        } else {
            self.io.poll_write_all_buf(&mut self.write_buffer, cx)
        }
    }
}

impl<IO> std::fmt::Debug for IoBuffer<IO> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("IoBuffer").finish_non_exhaustive()
    }
}

// ===== IoHandle =====

/// Handle for IO reading.
pub struct IoHandle {
    shared: Arc<Shared>,
}

impl IoHandle {
    pub fn poll_read(&mut self, _cx: &mut std::task::Context) -> Poll<io::Result<BytesMut>> {
        if !self.shared.wants_flag().poll_available() {
            return Poll::Pending;
        }
        Poll::Ready(self.shared.take_read_result())
    }
}

impl std::fmt::Debug for IoHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("IoHandle").finish_non_exhaustive()
    }
}
