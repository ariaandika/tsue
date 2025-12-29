use std::ptr::NonNull;
use std::sync::atomic::{AtomicU8, Ordering, fence};
use std::task::{Poll, Waker, ready};
use std::{io, mem};
use tcio::bytes::{BufMut, Bytes, BytesMut};
use tcio::io::AsyncIoRead;

use crate::body::coder::BodyCoder;
use crate::body::error::{ReadError, BodyError};

/// Sender shared handle.
pub struct SendHandle {
    inner: NonNull<SharedInner>,
}

/// Receiver shared handle.
pub struct RecvHandle {
    inner: NonNull<SharedInner>,
}

#[derive(Debug, Default)]
enum Data {
    #[default]
    None,
    Eof,
    Ok(Bytes),
    BodyErr(BodyError),
    IoErr(io::Error),
    // Recv(Waker),
    // Send(Waker),
}

// union Data {
//     bytes: std::mem::ManuallyDrop<Bytes>,
//     err: io::ErrorKind,
//     recv: std::mem::ManuallyDrop<Waker>,
//     send: std::mem::ManuallyDrop<Waker>,
// }

#[derive(Debug, Default)]
enum WakerHandle {
    #[default]
    None,
    Send(Waker),
    Recv(Waker),
}

impl WakerHandle {
    fn is_send(&self) -> bool {
        matches!(self, Self::Send(..))
    }

    fn is_recv(&self) -> bool {
        matches!(self, Self::Recv(..))
    }
}

struct SharedInner {
    /// send handle SHOULD NOT write data if DATA flag set
    /// send handle may only write data if DATA flag unset
    ///
    /// recv handle SHOULD NOT read data if DATA flag unset
    /// recv handle may only read data if DATA flag set
    ///
    /// either handle may read and write data if the SHARED flag unset
    data: Data,
    waker: WakerHandle,
    /// WANT: is handle wants data read
    /// SHARED: is both handle still alive
    /// DATA: is data available in shared memory
    ///
    /// [ .., is_data, is_shared, want ]
    flag: AtomicU8,
}
/// initial flag
const INITIAL_FLAG  : u8 = 0;
const WANT_MASK     : u8 = 1;
const SHARED_MASK   : u8 = 1 << 1;
const DATA_MASK     : u8 = 1 << 2;
// const RECV_MASK: u8     = 1 << 3;

unsafe impl Send for SendHandle { }
unsafe impl Sync for SendHandle { }
unsafe impl Send for RecvHandle { }
unsafe impl Sync for RecvHandle { }

impl Drop for SendHandle {
    fn drop(&mut self) {
        let mut inner = self.inner;

        // unset `shared` flag
        let old_flag = unsafe { inner.as_ref() }.flag.fetch_and(!SHARED_MASK, Ordering::Release);

        if old_flag.is_set::<SHARED_MASK>() {
            // recv handle is still alive

            if old_flag.is_set::<WANT_MASK>() {
                // if WANT flag is set, the memory is owned by the send handle and the recv handle
                // is idle, thus send handle must call waker if needed
                fence(Ordering::Acquire);
                let me = unsafe { inner.as_mut() };
                if let WakerHandle::Recv(waker) = mem::take(&mut me.waker) {
                    waker.wake();
                }
            }

            // otherwise, if WANT flag is unset, the send handle is idle and the memory is
            // owned by recv handle, thus nothing need to be done here

        } else {
            // recv handle already dropped, clean up the shared memory
            fence(Ordering::Acquire);
            unsafe { drop(Box::from_raw(inner.as_ptr())); }
        }
    }
}

impl Drop for RecvHandle {
    fn drop(&mut self) {
        let mut inner = self.inner;

        // unset `shared` flag
        let old_flag = unsafe { inner.as_ref() }.flag.fetch_and(!SHARED_MASK, Ordering::Release);

        if old_flag.is_set::<SHARED_MASK>() {
            // send handle is still alive

            if old_flag.is_set::<WANT_MASK>() {
                // if WANT flag is set, the memory is owned by the send handle and its the
                // one doing works, thus nothing need to be done here

            } else {
                // otherwise, if WANT flag is unset, the send handle is idle and the memory is
                // owned by recv handle, thus recv handle must call waker if needed
                fence(Ordering::Acquire);
                let me = unsafe { inner.as_mut() };
                if let WakerHandle::Send(waker) = mem::take(&mut me.waker) {
                    waker.wake();
                }
            }

        } else {
            // send handle already dropped, clean up the shared memory
            fence(Ordering::Acquire);
            unsafe { drop(Box::from_raw(inner.as_ptr())); }
        }
    }
}

impl RecvHandle {
    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<Result<Bytes, ReadError>> {
        let inner = unsafe { self.inner.as_mut() };

        // DATA WANT
        // 0    0       owned by Recv
        // 0    1       owned by Send
        // 1    0       owned by Recv
        // 1    1       INVALID

        // set `want` flag
        let flag = inner.flag.fetch_or(WANT_MASK, Ordering::Acquire);

        if flag.is_set::<DATA_MASK>() {
            // data is available

            let result = match mem::take(&mut inner.data) {
                Data::Ok(bytes) => Ok(bytes),
                Data::BodyErr(err) => Err(err.into()),
                Data::IoErr(err) => Err(err.into()),
                _ => unreachable!(),
            };

            debug_assert!(inner.waker.is_send());

            // unset the `data` and `want` flag
            inner.flag.store(flag & !DATA_MASK & !WANT_MASK, Ordering::Release);

            Poll::Ready(result)
        } else if flag.is_unset::<WANT_MASK>() {
            // `wants` is unset before, call waker and set current waker

            let inner = unsafe { self.inner.as_mut() };

            if flag.is_unset::<SHARED_MASK>() {
                // send handle is already dropped
                return Poll::Ready(Err(io::ErrorKind::ConnectionAborted.into()));
            }

            let waker = WakerHandle::Recv(cx.waker().clone());
            let waker = match mem::replace(&mut inner.waker, waker) {
                WakerHandle::Send(waker) => waker,
                data => {
                    unreachable!("data: {data:?}");
                }
            };
            waker.wake();

            Poll::Pending
        } else {
            // data not available
            // waker already called
            Poll::Pending
        }
    }
}

impl SendHandle {
    pub fn new() -> Self {
        let inner = SharedInner {
            flag: AtomicU8::new(INITIAL_FLAG),
            waker: WakerHandle::None,
            data: Data::None,
        };
        Self {
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(inner))) },
        }
    }

    pub fn handle(&mut self, cx: &mut std::task::Context) -> RecvHandle {
        // set `shared` flag
        let flag = unsafe { self.inner.as_ref() }.flag.fetch_or(SHARED_MASK, Ordering::Relaxed);

        // ensure no other handle are alive
        assert!(flag.is_unset::<SHARED_MASK>());

        // SAFETY: it is ensured that no other handle are holding the shared data
        let inner = unsafe { self.inner.as_mut() };

        inner.waker = WakerHandle::Send(cx.waker().clone());

        RecvHandle {
            inner: self.inner
        }
    }

    /// Check for data request from recv handle.
    ///
    /// Any IO error is propagated to recv handle.
    pub fn poll_read<IO>(
        &mut self,
        buf: &mut BytesMut,
        decoder: &mut BodyCoder,
        io: &mut IO,
        cx: &mut std::task::Context,
    ) -> Poll<()>
    where
        IO: AsyncIoRead,
    {
        let flag = unsafe { self.inner.as_ref() }.flag.load(Ordering::Acquire);

        if flag.is_unset::<SHARED_MASK>() {
            // SAFETY: recv handle is already dropped, no other handle is holding the memory
            unsafe {
                let inner = self.inner.as_mut();
                drop(mem::take(&mut inner.data));
                *inner.flag.get_mut() = INITIAL_FLAG;
            }
            return Poll::Ready(());
        }

        if flag.is_set::<WANT_MASK>() {
            // recv handle wants data

            debug_assert!(flag.is_unset::<DATA_MASK>());

            let result = loop {
                // TODO:(!) this will cause infinite loop when `decode_chunk` returns None
                if buf.is_empty() {
                    match ready!(io.poll_read_buf(buf, cx)) {
                        Ok(read) => {
                            if read == 0 {
                                todo!("connection terminated: {}", buf.spare_capacity_mut().len())
                            }
                            // ...
                        },
                        Err(err) => break Data::IoErr(err),
                    };
                }

                let Poll::Ready(data) = decoder.decode_chunk(buf) else {
                    continue;
                };
                break match data {
                    Some(Ok(data)) => Data::Ok(data.freeze()),
                    Some(Err(err)) => Data::BodyErr(err),
                    None => Data::Eof,
                }
            };

            // WANT flag is set, thus recv is idle and the data is owned by send handle
            let inner = unsafe { self.inner.as_mut() };
            inner.data = result;

            // wake recv handle
            let waker = WakerHandle::Send(cx.waker().clone());
            let WakerHandle::Recv(waker) = mem::replace(&mut inner.waker, waker) else {
                unreachable!();
            };
            waker.wake();

            // unset WANT and set DATA flag
            inner.flag.store(flag & !WANT_MASK | DATA_MASK, Ordering::Release);

            Poll::Ready(())
        } else {
            // recv handle is still alive
            // want flag is unset
            Poll::Ready(())
        }
    }
}

// ===== Helpers =====

impl std::fmt::Debug for SendHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shared").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for RecvHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").finish_non_exhaustive()
    }
}

trait Bitwise {
    fn is_set<const F: u8>(&self) -> bool;

    fn is_unset<const F: u8>(&self) -> bool;
}

impl Bitwise for u8 {
    #[inline(always)]
    fn is_set<const F: u8>(&self) -> bool {
        *self & F == F
    }

    #[inline(always)]
    fn is_unset<const F: u8>(&self) -> bool {
        *self & F != F
    }
}

