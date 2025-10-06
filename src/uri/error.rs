
/// A possible error value when parsing URI.
#[derive(Clone)]
pub enum UriError {
    /// Bytes ends before all components parsed.
    Incomplete,
    /// Bytes length is too large.
    TooLong,
    /// Invalid character found.
    Char,
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
    Incomplete => "URI incomplete",
    TooLong => "URI too long",
    Char => "URI contains invalid character",
}

impl std::error::Error for UriError { }

impl std::fmt::Debug for UriError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\"{self}\"")
    }
}
