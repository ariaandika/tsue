use std::ptr::NonNull;
use std::sync::atomic::{AtomicU8, Ordering};


#[derive(Debug)]
pub struct Shared {
    inner: NonNull<SharedInner>,
}

pub struct SharedHandle {
    flag: flag::AtomicFlag,
}

struct SharedInner {
    /// [ .., is_drop, is_wants ]
    flag: AtomicU8,
}

const IDLE_FLAG: u8 = 0x00;
const WANT_MASK: u8 = 0x01;
const DROP_MASK: u8 = 0x10;

unsafe impl Send for Shared { }
unsafe impl Sync for Shared { }

impl Drop for Shared {
    fn drop(&mut self) {
        unsafe { self.inner.as_mut().release() };
    }
}

impl SharedInner {
    fn load_flag(&self) -> (bool, bool) {
        let flag = self.flag.load(Ordering::Relaxed);
        (flag & WANT_MASK != 0, flag & DROP_MASK != 0)
    }

    fn release(&mut self) {
        // set `drop` flag
        let flag = self.flag.fetch_or(DROP_MASK, Ordering::Release);

        if flag & DROP_MASK != DROP_MASK {
            return;
        }

        self.flag.load(Ordering::Acquire);

        // other handle already dropped, clean up the memory
        unsafe { drop(Box::from_raw(self as *mut Self)) };
    }
}

impl Shared {
    pub fn new() -> Self {
        let inner = SharedInner {
            flag: AtomicU8::new(IDLE_FLAG),
        };
        Self {
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(inner))) },
        }
    }

    /// Returns `true` if handle wants data.
    pub fn poll_wants(&mut self, cx: &mut std::task::Context) {}
}

mod flag {
    use std::mem;
    use std::sync::atomic::{AtomicU8, Ordering};

    #[derive(Debug)]
    #[repr(transparent)]
    pub struct AtomicFlag(AtomicU8);

    impl AtomicFlag {
        pub fn new(init: u8) -> Self {
            Self(AtomicU8::new(init))
        }

        pub fn load(&self) -> Flag {
            unsafe { mem::transmute::<u8, Flag>(self.0.load(Ordering::Relaxed)) }
        }
    }

    #[derive(Debug)]
    #[repr(u8)]
    pub enum Flag {
        /// Nothing in the buffer.
        Idle,
        /// User wants a IO read data.
        Wants,
        /// Data available in the buffer.
        Data,
        /// The IO has been dropped, the handle is responsible for the memory.
        IODrop,
        /// The handle has been dropped, the IO is responsible for the memory.
        HandleDrop,
    }
}

