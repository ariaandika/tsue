
/// A possible error value when parsing URI.
#[derive(Clone)]
pub enum UriError {
    /// Bytes length is too large.
    TooLong,
    InvalidScheme,
    InvalidAuthority,
    InvalidPath,
}

// ===== Error =====

macro_rules! gen_error {
    ($($variant:pat => $msg:literal),* $(,)?) => {
        impl UriError {
            pub(crate) const fn panic_const(&self) -> ! {
                use UriError::*;
                match self {
                    $($variant => panic!($msg),)*
                }
            }
        }

        impl std::fmt::Display for UriError {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                use UriError::*;
                match self {
                    $($variant => f.write_str($msg),)*
                }
            }
        }
    };
}

gen_error! {
    TooLong => "URI too long",
    InvalidScheme => "invalid scheme",
    InvalidAuthority => "invalid authority",
    InvalidPath => "invalid path",
}

impl std::error::Error for UriError { }

impl std::fmt::Debug for UriError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\"{self}\"")
    }
}
