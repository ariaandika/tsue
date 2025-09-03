use std::{
    cmp, io, mem,
    sync::{
        Arc, Mutex,
        atomic::{
            AtomicU8,
            Ordering::{Relaxed, Release},
        },
    },
    task::{Poll, ready},
};
use tcio::{
    bytes::BytesMut,
    io::{AsyncIoRead, AsyncIoWrite},
};

use crate::body::Body;

pub(crate) struct IoBuffer<IO> {
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

    /// Write buffered
    pub fn write(&mut self, buf: &[u8]) {
        self.write_buffer.extend_from_slice(buf);
    }

    pub fn set_remaining(&mut self, remaining: u64) {
        self.remaining = remaining;
    }

    pub fn get_handle(&self) -> IoHandle {
        IoHandle {
            shared: self.shared.clone(),
        }
    }

    pub(crate) fn write_body(&self, body: Body) -> BodyWrite {
        BodyWrite::new(body)
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
    /// Returns `true` if data is available, `Service` should be polled again immediately.
    ///
    /// This should be polled at the same time with `Service` which holds [`IoHandle`].
    pub(crate) fn poll_io_wants(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<bool>> {
        let Some(wants) = self.shared.wants.load_is_want() else {
            // `IoHandle` have not been called yet
            return Poll::Ready(Ok(false));
        };

        let poll = if WantsFlag::is_want_read(wants) {
            self.poll_read_remaining(cx)
        } else {
            assert!(WantsFlag::is_want_collect(wants));
            self.poll_collect(cx)
        };

        ready!(poll)?;
        self.shared.wants.set_available();
        Poll::Ready(Ok(true))
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
        self.shared.wants.set_idle();

        Poll::Ready(Ok(()))
    }
}

impl<IO> IoBuffer<IO>
where
    IO: AsyncIoWrite,
{
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

// ===== BodyWriteFuture =====

pub(crate) struct BodyWrite {
    body: Body,
    phase: Phase,
}

enum Phase {
    Read,
    Write(tcio::bytes::Bytes),
}

impl BodyWrite {
    pub fn new(body: Body) -> Self {
        Self {
            body,
            phase: Phase::Read,
        }
    }

    pub fn poll_write<IO: AsyncIoWrite>(
        &mut self,
        io: &mut IoBuffer<IO>,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<()>> {
        loop {
            match &mut self.phase {
                Phase::Read => {
                    if self.body.has_remaining() {
                        let data = ready!(self.body.poll_read(cx))?;
                        self.phase = Phase::Write(data);
                    } else {
                        break;
                    }
                }
                Phase::Write(bytes) => {
                    ready!(io.io.poll_write(bytes, cx))?;
                    bytes.clear();
                    if bytes.is_empty() {
                        self.phase = Phase::Read;
                    }
                }
            }
        }
        Poll::Ready(Ok(()))
    }
}

// ===== Shared =====

/// Shared state for [`IoBuffer`] and [`IoHandle`].
struct Shared {
    wants: WantsFlag,
    /// this Mutex SHOULD never block,
    /// because `IoBuffer` and `IoHandle`
    /// SHOULD be in the same task
    ///
    /// if user send `IoHandle` to other task,
    /// it would be considered an error,
    /// because the main task may return before the
    /// spawned task end, the io have been drained,
    /// and currently writing response
    ///
    /// HTTP/1.1 cannot handle multiple stream
    read: Mutex<io::Result<BytesMut>>,
}

impl Shared {
    fn new() -> Self {
        Self {
            wants: WantsFlag::new(),
            read: Mutex::new(Ok(BytesMut::new())),
        }
    }

    fn try_lock(&self) -> io::Result<std::sync::MutexGuard<'_, Result<BytesMut, io::Error>>> {
        match self.read.try_lock() {
            Ok(ok) => Ok(ok),
            Err(std::sync::TryLockError::Poisoned(ok)) => Ok(ok.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => {
                Err(io::Error::other("cannot acquire io lock"))
            }
        }
    }

    fn take_read_result(&self) -> io::Result<BytesMut> {
        mem::replace(&mut *self.try_lock()?, Ok(BytesMut::new()))
    }

    fn set_read_result(&self, result: io::Result<BytesMut>) -> io::Result<()> {
        *self.try_lock()? = result;
        Ok(())
    }
}

// ===== IoHandle =====

/// Handle for IO reading.
pub struct IoHandle {
    shared: Arc<Shared>,
}

impl IoHandle {
    pub fn poll_read(&mut self, _cx: &mut std::task::Context) -> Poll<io::Result<BytesMut>> {
        if !self.shared.wants.poll_available() {
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

// ===== Wants =====

/// Flag indicating when [`IoHandle`] wants a data.
#[derive(Debug)]
struct WantsFlag {
    flag: AtomicU8,
}

#[rustfmt::skip]
impl WantsFlag {
    //                     [  RW   AW]
    const IDLE: u8      = 0b0000_0000;
    const WANT: u8      = 0b0000_0001;
    const AVAILABLE: u8 = 0b0000_0010;

    const WANT_MASK: u8 = 0b0000_0001;
    const WANT_READ: u8 = 0b0000_0001;
    const WANT_COLL: u8 = 0b0001_0001; // WANT_COLLECT

    #[cfg(debug_assertions)]
    const WANT_KIND: u8 = 0b0001_0001;
}

impl WantsFlag {
    fn new() -> Self {
        Self {
            flag: AtomicU8::new(Self::IDLE),
        }
    }

    /// Returns `true` if data is available.
    fn poll_available(&self) -> bool {
        let Err(flag) = self
            .flag
            .compare_exchange(Self::IDLE, Self::WANT, Release, Relaxed)
        else {
            // previously, flag is `IDLE`, and now set to `WANT`
            return false;
        };

        // flag is NOT `IDLE`, we continue
        debug_assert_ne!(flag, Self::IDLE);

        if let Err(current) =
            self.flag
                .compare_exchange(Self::AVAILABLE, Self::IDLE, Release, Relaxed)
        {
            debug_assert_eq!(current, Self::WANT);
            // flag is still `WANT`, wait for `AVAILABLE`
            return false;
        }

        // now flag is `AVAILABLE`, and we set it to `IDLE`
        true
    }

    fn is_want_read(wants: u8) -> bool {
        debug_assert_eq!(
            (wants | Self::WANT_KIND),
            Self::WANT_KIND,
            "`is_want_read` should only used in WANT flags"
        );
        wants == Self::WANT_READ
    }

    fn is_want_collect(wants: u8) -> bool {
        debug_assert_eq!(
            (wants | Self::WANT_KIND),
            Self::WANT_KIND,
            "`is_want_read` should only used in WANT flags"
        );
        wants == Self::WANT_COLL
    }

    fn load_is_want(&self) -> Option<u8> {
        let flag = self.flag.load(Relaxed);
        ((flag & Self::WANT_MASK) == 1).then_some(flag)
    }

    // fn is_available(&self) -> bool {
    //     self.flag.load(Relaxed) == Self::AVAILABLE
    // }

    fn set_idle(&self) {
        self.flag.store(Self::IDLE, Release)
    }

    fn set_available(&self) {
        assert_eq!(
            self.flag.swap(Self::AVAILABLE, Release),
            Self::WANT,
            "wants flag `AVAILABLE` should only set when flag is `WANT`"
        )
    }
}
