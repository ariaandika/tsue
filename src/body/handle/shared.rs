use std::ptr::NonNull;
use std::sync::atomic::{AtomicU8, Ordering, fence};
use std::task::{Poll, Waker};
use std::{io, mem};
use tcio::bytes::{Bytes, BytesMut};
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
}

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

    // fn is_recv(&self) -> bool {
    //     matches!(self, Self::Recv(..))
    // }
}

struct SharedInner {
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
        // unset SHARED flag
        let old_flag = unsafe { self.inner.as_ref() }
            .flag
            .fetch_and(!SHARED_MASK, Ordering::Release);

        if old_flag.is_set::<SHARED_MASK>() {
            // recv handle is still alive

            if old_flag.is_set::<WANT_MASK>() {
                // if WANT flag is set, the memory is owned by the send handle and the recv handle
                // is idle, thus send handle must call waker if needed
                fence(Ordering::Acquire);
                if let WakerHandle::Recv(waker) =
                    mem::take(unsafe { &mut self.inner.as_mut().waker })
                {
                    waker.wake();
                }
            }

            // otherwise, if WANT flag is unset, the send handle is idle and the memory is
            // owned by recv handle, thus nothing need to be done here
        } else {
            // recv handle already dropped, clean up the shared memory
            fence(Ordering::Acquire);
            unsafe {
                drop(Box::from_raw(self.inner.as_ptr()));
            }
        }
    }
}

impl Drop for RecvHandle {
    fn drop(&mut self) {
        // unset SHARED flag
        let old_flag = unsafe { self.inner.as_ref() }
            .flag
            .fetch_and(!SHARED_MASK, Ordering::Release);

        if old_flag.is_set::<SHARED_MASK>() {
            // send handle is still alive

            if old_flag.is_set::<WANT_MASK>() {
                // if WANT flag is set, the memory is owned by the send handle and its the
                // one doing works, thus nothing need to be done here
            } else {
                fence(Ordering::Acquire);
                // otherwise, if WANT flag is unset, the send handle is idle and the memory is
                // owned by recv handle, thus recv handle must call waker if needed
                let inner = unsafe { self.inner.as_mut() };

                if let WakerHandle::Send(waker) = mem::take(&mut inner.waker) {
                    waker.wake();
                }

                // reset the state
                mem::take(&mut inner.data);
                mem::take(&mut inner.waker);
                inner.flag.store(INITIAL_FLAG, Ordering::Release);
            }
        } else {
            // send handle already dropped, clean up the shared memory
            fence(Ordering::Acquire);
            unsafe {
                drop(Box::from_raw(self.inner.as_ptr()));
            }
        }
    }
}

impl RecvHandle {
    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<Option<Result<Bytes, ReadError>>> {
        let flag = unsafe { self.inner.as_ref() }.flag.load(Ordering::Relaxed);

        // DATA is unset, WANT is set
        if flag & (DATA_MASK | WANT_MASK) == WANT_MASK {
            // recv already set WANT flag but no data is available yet
            return Poll::Pending;
        }

        fence(Ordering::Acquire);

        debug_assert!(flag.is_unset::<WANT_MASK>());

        // WANT should be unset, which implied that memory is owned by recv handle
        let inner = unsafe { self.inner.as_mut() };

        if flag.is_set::<DATA_MASK>() {
            // data is available

            let result = match mem::take(&mut inner.data) {
                Data::Ok(bytes) => Ok(bytes),
                Data::BodyErr(err) => Err(err.into()),
                Data::IoErr(err) => Err(err.into()),
                Data::Eof => {
                    // immediately return, leave the DATA flag set, because no more remaining data
                    // need to be read
                    return Poll::Ready(None);
                }
                Data::None => unreachable!("Data::None with DATA flag set"),
            };

            debug_assert!(inner.waker.is_send());

            // unset the DATA flag
            inner.flag.store(flag & !DATA_MASK, Ordering::Release);

            Poll::Ready(Some(result))

        } else {
            // call waker and set current waker

            if flag.is_unset::<SHARED_MASK>() {
                // send handle is already dropped
                return Poll::Ready(Some(Err(io::ErrorKind::ConnectionAborted.into())));
            }

            let waker = WakerHandle::Recv(cx.waker().clone());
            let WakerHandle::Send(waker) = mem::replace(&mut inner.waker, waker) else {
                unreachable!("desynchronized waker handle");
            };
            waker.wake();

            // set WANT flag
            inner.flag.store(flag | WANT_MASK, Ordering::Release);

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
    )
    where
        IO: AsyncIoRead,
    {
        let flag = unsafe { self.inner.as_ref() }.flag.load(Ordering::Relaxed);

        // DATA is set, or WANT is unset
        if flag.is_set::<DATA_MASK>() | flag.is_unset::<WANT_MASK>() {
            // data is still available, or recv have not set WANT flag yet
            return;
        }

        if flag.is_unset::<SHARED_MASK>() {
            // SAFETY: recv handle is already dropped, no other handle is holding the memory
            unsafe {
                let inner = self.inner.as_mut();
                drop(mem::take(&mut inner.data));
                *inner.flag.get_mut() = INITIAL_FLAG;
            }
            return;
        }

        fence(Ordering::Acquire);

        // recv handle wants data
        debug_assert!(flag.is_set::<WANT_MASK>());

        let result = loop {
            break match decoder.decode_chunk(buf) {
                Poll::Ready(Some(Ok(data))) => Data::Ok(data.freeze()),
                Poll::Ready(Some(Err(err))) => Data::BodyErr(err),
                Poll::Ready(None) => Data::Eof,
                Poll::Pending => {
                    let Poll::Ready(result) = io.poll_read_buf(buf, cx) else {
                        return;
                    };
                    match result {
                        Ok(read) => {
                            if read == 0 {
                                break Data::IoErr(io::ErrorKind::ConnectionAborted.into());
                            }
                            continue;
                        },
                        Err(err) => Data::IoErr(err),
                    }
                }
            }
        };

        // WANT flag is set, thus recv is idle and the data is owned by send handle
        let inner = unsafe { self.inner.as_mut() };
        inner.data = result;

        // wake recv handle
        let waker = WakerHandle::Send(cx.waker().clone());
        let WakerHandle::Recv(waker) = mem::replace(&mut inner.waker, waker) else {
            unreachable!("desynchronized waker handle");
        };
        waker.wake();

        // unset WANT and set DATA flag
        inner.flag.store(flag & !WANT_MASK | DATA_MASK, Ordering::Release);
    }

    /// Wait for recv handle to be dropped.
    ///
    /// Otherwise supply data by calling `SendHandle::poll_read`.
    pub fn poll_close<IO: AsyncIoRead>(
        &mut self,
        buf: &mut BytesMut,
        decoder: &mut BodyCoder,
        io: &mut IO,
        cx: &mut std::task::Context,
    ) -> Poll<()> {
        let flag = unsafe { self.inner.as_ref() }.flag.load(Ordering::Acquire);

        if flag.is_set::<SHARED_MASK>() {
            self.poll_read(buf, decoder, io, cx);
            Poll::Pending

        } else {
            // recv handle is already dropped, and the state should be reset
            // TODO: drain request body
            loop {
                match decoder.decode_chunk(buf) {
                    Poll::Ready(Some(Ok(_))) => {}
                    Poll::Ready(Some(Err(_))) => todo!("body error when draining"),
                    Poll::Ready(None) => break,
                    Poll::Pending => {
                        let Poll::Ready(result) = io.poll_read_buf(buf, cx) else {
                            return Poll::Pending;
                        };
                        match result {
                            Ok(0) => break,
                            Ok(_) => { },
                            Err(_) => todo!("io error when draining"),
                        }
                    }
                }
            }
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

// Other form of the shared implementation
//
// Simpler but slightly wasted syscall for unread body
//
// Send and Recv handle holds shared memory
//
// One of handle "owned" the memory at a time, denoted by atomic flag
//
// Waker is by the opposite handle who own the memory
//
// Send poll:
// - if owned, poll io read for data
// - on read ready, write data, call and replace waker, set owend flag to recv
// - if not owned, do nothing
//
// Recv poll:
// - if owned,
//   - if data, take data, set owned flag to send
//   - else, call and replace waker, set owned flag to send
// - if not owned, returns pending
