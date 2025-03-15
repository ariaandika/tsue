//! the [`StatusCode`] struct
use std::{
    fmt::{Debug, Display, Formatter},
    num::NonZeroU16,
};

/// an http status code
#[derive(Clone, Copy)]
pub struct StatusCode(NonZeroU16);

impl Display for StatusCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.status_str())?;
        f.write_str(" ")?;
        f.write_str(self.message())
    }
}

impl Debug for StatusCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StatusCode").field(&self.0.get()).finish()
    }
}

impl Default for StatusCode {
    fn default() -> Self {
        Self::OK
    }
}

macro_rules! status_code_v2 {
    (@msgs $($int:literal $msg:literal,)*) => {
        impl StatusCode {
            /// return status and reason, e.g: `"200 OK"`
            pub fn as_bytes(&self) -> &'static [u8] {
                match self.0.get() {
                    $(
                        $int => concat!(stringify!($int)," ",$msg).as_bytes(),
                    )*
                    _ => unreachable!(),
                }
            }

            /// return status code as str, e.g: `"200"`
            pub fn status_str(&self) -> &'static str {
                match self.0.get() {
                    $(
                        $int => stringify!($int),
                    )*
                    _ => unreachable!(),
                }
            }

            /// return status message, e.g: `"OK"`
            pub fn message(&self) -> &'static str {
                match self.0.get() {
                    $(
                        $int => $msg,
                    )*
                    _ => unreachable!(),
                }
            }
        }
    };
    (@code $($int:literal $name:ident,)*) => {
        impl StatusCode {
            $(
                pub const $name: Self = Self(unsafe { NonZeroU16::new_unchecked($int) });
            )*
        }
    };

    (@
        (msg => $($msgs:tt)*)
        (code => $($codes:tt)*)
    ) => {
        status_code_v2!(@code $($codes)*);
        status_code_v2!(@msgs $($msgs)*);
    };

    (@
        (msg => $($msgs:tt)*)
        (code => $($codes:tt)*)
        $int:literal $name:ident $msg:literal, $($tt:tt)*
    ) => {
        status_code_v2! {@
            (msg => $($msgs)* $int $msg,)
            (code => $($codes)* $int $name,)
            $($tt)*
        }
    };

    ($int:literal $($tt:tt)*) => {
        status_code_v2! {@
            (msg => )
            (code => )
            $int $($tt)*
        }
    };
}

status_code_v2! {
    200 OK "OK",
    400 BAD_REQUEST "Bad Request",
    404 NOT_FOUND "Not Found",
    405 METHOD_NOT_ALLOWED "Method Not Allowed",
}

