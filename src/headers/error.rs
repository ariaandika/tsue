//! Error types that can occur during header related operation.

/// An error that can occur in header related operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderError {
    /// Bytes is empty.
    Empty,
    /// Bytes too long.
    TooLong,
    /// Bytes contains invalid character.
    Invalid,
}

impl HeaderError {
    pub(crate) const fn invalid_len(len: usize) -> Self {
        match len {
            0 => Self::Empty,
            _ => Self::TooLong,
        }
    }

    pub(crate) const fn message(&self) -> &'static str {
        match self {
            Self::Empty => "cannot be empty",
            Self::TooLong => "too long",
            Self::Invalid => "contains invalid byte",
        }
    }

    pub(crate) const fn panic_const(self) -> ! {
        panic!("{}",self.message())
    }
}

impl std::error::Error for HeaderError {}
impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

// ===== Reserve Error =====

/// An error that can occur when performing allocation in [`HeaderMap`].
///
/// [`HeaderMap`]: crate::headers::HeaderMap
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TryReserveError { }

impl std::error::Error for TryReserveError {}

impl std::fmt::Display for TryReserveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("header map capacity exceeded")
    }
}
