//! HPACK Related Error.
use crate::h2::hpack::huffman::HuffmanError;
use crate::headers::error::{HeaderError, TryReserveError};

/// HPACK Related Error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HpackError {
    /// Bytes is insufficient.
    Incomplete,
    /// Bytes is too large.
    TooLarge,
    /// Header fields is too many.
    TooMany,
    /// Invalid use of index 0.
    ZeroIndex,
    /// Indexed header not found.
    NotFound,
    /// Huffman coding error.
    Huffman,
    /// Header validation error.
    InvalidHeader,
    /// Pseudo header is not at the beginning of header block.
    InvalidPseudoHeader,
    /// Size update is too large or is not at the beginning of header block.
    InvalidSizeUpdate,
}

impl std::error::Error for HpackError { }
impl std::fmt::Display for HpackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incomplete => f.write_str("bytes incomplete"),
            Self::TooLarge => f.write_str("bytes too large"),
            Self::TooMany => f.write_str("header too many"),
            Self::ZeroIndex => f.write_str("invalid use of index 0"),
            Self::NotFound => f.write_str("indexed header not found"),
            Self::Huffman => f.write_str("huffman coding error"),
            Self::InvalidHeader => f.write_str("invalid header"),
            Self::InvalidPseudoHeader => f.write_str("missplaced pseudo header"),
            Self::InvalidSizeUpdate => f.write_str("invalid size update"),
        }
    }
}

impl From<HeaderError> for HpackError {
    #[inline]
    fn from(err: HeaderError) -> Self {
        match err {
            HeaderError::Empty => Self::Incomplete,
            HeaderError::TooLong => Self::TooLarge,
            HeaderError::Invalid => Self::InvalidHeader,
        }
    }
}

impl From<HuffmanError> for HpackError {
    #[inline]
    fn from(_: HuffmanError) -> Self {
        Self::Huffman
    }
}

impl From<TryReserveError> for HpackError {
    #[inline]
    fn from(_: TryReserveError) -> Self {
        Self::TooMany
    }
}
