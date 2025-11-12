use std::{
    io, mem,
    sync::{
        Mutex,
        atomic::{
            AtomicU8,
            Ordering::{Relaxed, Release},
        },
    },
};
use tcio::bytes::BytesMut;

/// Shared state for [`IoBuffer`] and [`IoHandle`].
pub struct Shared {
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
    pub fn new() -> Self {
        Self {
            wants: WantsFlag::new(),
            read: Mutex::new(Ok(BytesMut::new())),
        }
    }

    pub fn take_read_result(&self) -> io::Result<BytesMut> {
        mem::replace(&mut *self.try_lock()?, Ok(BytesMut::new()))
    }

    pub fn set_read_result(&self, result: io::Result<BytesMut>) -> io::Result<()> {
        *self.try_lock()? = result;
        Ok(())
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

    #[inline]
    pub fn wants_flag(&self) -> &WantsFlag {
        &self.wants
    }
}

// ===== WantsFlag =====

/// Flag indicating when [`IoHandle`] wants a data.
#[derive(Debug)]
pub struct WantsFlag {
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
    pub fn poll_available(&self) -> bool {
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

    pub fn is_want_read(wants: u8) -> bool {
        debug_assert_eq!(
            (wants | Self::WANT_KIND),
            Self::WANT_KIND,
            "`is_want_read` should only used in WANT flags"
        );
        wants == Self::WANT_READ
    }

    pub fn is_want_collect(wants: u8) -> bool {
        debug_assert_eq!(
            (wants | Self::WANT_KIND),
            Self::WANT_KIND,
            "`is_want_read` should only used in WANT flags"
        );
        wants == Self::WANT_COLL
    }

    pub fn load_is_want(&self) -> Option<u8> {
        let flag = self.flag.load(Relaxed);
        ((flag & Self::WANT_MASK) == 1).then_some(flag)
    }

    // fn is_available(&self) -> bool {
    //     self.flag.load(Relaxed) == Self::AVAILABLE
    // }

    pub fn set_idle(&self) {
        self.flag.store(Self::IDLE, Release)
    }

    pub fn set_available(&self) {
        assert_eq!(
            self.flag.swap(Self::AVAILABLE, Release),
            Self::WANT,
            "wants flag `AVAILABLE` should only set when flag is `WANT`"
        )
    }
}
