pub use crate::headers::name::HeaderNameError;
pub use crate::headers::value::HeaderValueError;

/// An error that can occur in header related operations.
#[derive(Debug)]
pub enum HeaderError {
    /// Header name parsing error.
    Name(HeaderNameError),
    /// Header value parsing error.
    Value(HeaderValueError),
}

impl HeaderError {
    pub(crate) const fn message(&self) -> &'static str {
        match self {
            Self::Name(err) => err.message(),
            Self::Value(err) => err.message(),
        }
    }
}

impl From<HeaderNameError> for HeaderError {
    #[inline]
    fn from(v: HeaderNameError) -> Self {
        Self::Name(v)
    }
}

impl From<HeaderValueError> for HeaderError {
    #[inline]
    fn from(v: HeaderValueError) -> Self {
        Self::Value(v)
    }
}

impl std::error::Error for HeaderError {}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message().fmt(f)
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
