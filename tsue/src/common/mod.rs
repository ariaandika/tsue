//! Utility types.
mod bytestr;

pub use bytestr::ByteStr;

macro_rules! log {
    ($($tt:tt)*) => {
        {
            #[cfg(feature = "log")]
            log::error!($($tt)*);
            #[cfg(not(feature = "log"))]
            eprintln!($($tt)*);
        }
    };
}

pub(crate) use log;
