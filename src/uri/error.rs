/// An error that can occur during URI validation.
#[derive(Debug, Clone)]
pub enum UriError {
    /// Excessive bytes length.
    ExcessiveBytes,
    /// Invalid scheme.
    InvalidScheme,
    /// Invalid authority.
    InvalidAuthority,
    /// Invalid path.
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
                    $($variant => $msg.fmt(f),)*
                }
            }
        }
    };
}

gen_error! {
    ExcessiveBytes => "excessive bytes length",
    InvalidScheme => "invalid scheme",
    InvalidAuthority => "invalid authority",
    InvalidPath => "invalid path",
}

impl std::error::Error for UriError { }
