use std::ptr::NonNull;
use std::sync::atomic::{AtomicU8, Ordering, fence};
use std::task::{Poll, Waker, ready};
use std::{io, mem};
use tcio::bytes::{Bytes, BytesMut};
use tcio::io::AsyncIoRead;

/// Sender shared handle.
pub struct Shared {
    inner: NonNull<SharedInner>,
}

/// Receiver shared handle.
pub struct Handle {
    inner: NonNull<SharedInner>,
}

#[derive(Default)]
enum Data {
    #[default]
    None,
    Ok(Bytes),
    Err(io::Error),
    Recv(Waker),
    Send(Waker),
}

// union Data {
//     bytes: std::mem::ManuallyDrop<Bytes>,
//     err: io::ErrorKind,
//     recv: std::mem::ManuallyDrop<Waker>,
//     send: std::mem::ManuallyDrop<Waker>,
// }

struct SharedInner {
    /// sender handle SHOULD NOT write data if DATA flag set
    /// sender handle may only write data if DATA flag unset
    ///
    /// receiver handle SHOULD NOT read data if DATA flag unset
    /// receiver handle may only read data if DATA flag set
    ///
    /// either handle may read and write data if the SHARED flag unset
    data: Data,
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

unsafe impl Send for Shared { }
unsafe impl Sync for Shared { }
unsafe impl Send for Handle { }
unsafe impl Sync for Handle { }

impl Drop for Shared {
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
                if let Data::Recv(waker) = mem::take(&mut me.data) {
                    waker.wake();
                }
            }

            // otherwise, if WANT flag is unset, the send handle is idle and the memory is
            // owned by recv handle, thus recv handle must call waker if needed

        } else {
            // receiver handle already dropped, clean up the shared memory
            fence(Ordering::Acquire);
            unsafe { drop(Box::from_raw(inner.as_ptr())); }
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let mut inner = self.inner;

        // unset `shared` flag
        let old_flag = unsafe { inner.as_ref() }.flag.fetch_and(!SHARED_MASK, Ordering::Release);

        if old_flag.is_set::<SHARED_MASK>() {
            // send handle is still alive

            if old_flag.is_set::<WANT_MASK>() {
                // if WANT flag is set, the memory is owned by the send handle and its the
                // one doing works, thus no waker needs to be called

            } else {
                // otherwise, if WANT flag is unset, the send handle is idle and the memory is
                // owned by recv handle, thus recv handle must call waker if needed
                fence(Ordering::Acquire);
                let me = unsafe { inner.as_mut() };
                if let Data::Send(waker) = mem::take(&mut me.data) {
                    waker.wake();
                }
            }

        } else {
            // receiver handle already dropped, clean up the shared memory
            fence(Ordering::Acquire);
            unsafe { drop(Box::from_raw(inner.as_ptr())); }
        }
    }
}

impl Handle {
    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<Bytes>> {
        let inner = unsafe { self.inner.as_mut() };

        // set `wants` flag
        let flag = inner.flag.fetch_or(WANT_MASK, Ordering::Acquire);

        if flag.is_unset::<SHARED_MASK>() {
            // sender handle is already dropped
            return Poll::Ready(Err(io::ErrorKind::ConnectionAborted.into()));
        }

        if flag.is_set::<DATA_MASK>() {
            // data is available

            let result = match mem::replace(&mut inner.data, Data::None) {
                Data::Ok(bytes) => Ok(bytes),
                Data::Err(err) => Err(err),
                _ => unreachable!(),
            };

            // unset the `data` flag
            inner.flag.store(flag & !DATA_MASK, Ordering::Release);

            Poll::Ready(result)
        } else if flag.is_unset::<WANT_MASK>() {
            // `wants` is unset before, call waker and set current waker

            let inner = unsafe { self.inner.as_mut() };
            let waker = Data::Recv(cx.waker().clone());
            let Data::Send(waker) = mem::replace(&mut inner.data, waker) else {
                unreachable!();
            };
            waker.wake();

            Poll::Pending
        } else {
            // sender handle is still alive
            // data not available
            // waker already called
            Poll::Pending
        }
    }
}

impl Shared {
    pub fn new() -> Self {
        let inner = SharedInner {
            flag: AtomicU8::new(INITIAL_FLAG),
            data: Data::None,
        };
        Self {
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(inner))) },
        }
    }

    pub fn handle(&mut self, cx: &mut std::task::Context) -> Handle {
        // set `shared` flag
        let flag = unsafe { self.inner.as_ref().flag.fetch_or(SHARED_MASK, Ordering::Relaxed) };

        // ensure no other handle are alive
        assert!(flag.is_unset::<SHARED_MASK>());

        // SAFETY: it is ensured that no other handle are holding the shared data
        let inner = unsafe { self.inner.as_mut() };

        inner.data = Data::Send(cx.waker().clone());

        Handle {
            inner: self.inner
        }
    }

    /// Check for data request from recv handle.
    ///
    /// Any IO error is propagated to recv handle.
    pub fn poll_read<IO>(&mut self, buf: &mut BytesMut, io: &mut IO, cx: &mut std::task::Context) -> Poll<()>
    where
        IO: AsyncIoRead,
    {
        let flag = unsafe { self.inner.as_ref() }.flag.load(Ordering::Acquire);

        if flag.is_unset::<SHARED_MASK>() {
            // recv handle is already dropped
            Poll::Ready(())

        } else if flag.is_set::<DATA_MASK>() {
            // data is still available
            Poll::Ready(())

        } else if flag.is_set::<WANT_MASK>() {
            // recv handle wants data
            let result = match ready!(io.poll_read_buf(buf, cx)) {
                Ok(read) => {
                    if read == 0 {
                        todo!("connection terminated")
                    }
                    Data::Ok(buf.split_to(read).freeze())
                },
                Err(err) => Data::Err(err),
            };

            // WANT flag is set, thus recv is idle and the data is owned by send handle
            let inner = unsafe { self.inner.as_mut() };
            let Data::Send(waker) = mem::replace(&mut inner.data, result) else {
                unreachable!();
            };
            // unset WANT and set DATA flag
            inner.flag.store(flag & !WANT_MASK & DATA_MASK, Ordering::Release);
            waker.wake();

            Poll::Ready(())

        } else {
            // recv handle is still alive
            // want flag is unset
            Poll::Pending
        }
    }
}

// ===== Helpers =====

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shared").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Handle {
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
