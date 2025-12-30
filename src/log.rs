#![allow(unused, reason = "logger")]

macro_rules! info {
    ($($tt:tt)*) => {
        #[cfg(feature = "log")]
        ::log::info!($($tt)*);
    };
}

macro_rules! debug {
    ($($tt:tt)*) => {
        #[cfg(feature = "log")]
        ::log::debug!($($tt)*);
    };
}

macro_rules! warning {
    ($($tt:tt)*) => {
        #[cfg(feature = "log")]
        ::log::warn!($($tt)*);
    };
}

macro_rules! error {
    ($($tt:tt)*) => {
        #[cfg(feature = "log")]
        ::log::error!($($tt)*);
    };
}

pub(crate) use {info, debug, warning, error};
