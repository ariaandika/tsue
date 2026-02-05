//! HPACK Related Error.
use crate::h2::hpack::huffman::HuffmanError;
use crate::headers::error::{HeaderError, TryReserveError};

/// HPACK Related Error.
#[derive(Debug)]
pub enum HpackError {
    /// Bytes is insufficient.
    Incomplete,
    /// Header fields is too many.
    TooMany,
    /// Unknown header block kind.
    UnknownRepr,
    /// Found `0` index.
    ZeroIndex,
    /// Indexed header not found.
    NotFound,
    /// Huffman coding error.
    Huffman,
    /// Pseudo header is not at the beginning of header block.
    InvalidPseudoHeader,
    /// Size update is too large or is not at the beginning of header block.
    InvalidSizeUpdate,
    /// Header name or value validation error.
    InvalidHeader(HeaderError),
}

impl std::error::Error for HpackError { }
impl std::fmt::Display for HpackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incomplete => f.write_str("bytes incomplete"),
            Self::TooMany => f.write_str("header too many"),
            Self::UnknownRepr => f.write_str("unknown header field representation"),
            Self::ZeroIndex => f.write_str("invalid 0 index"),
            Self::NotFound => f.write_str("indexed header not found"),
            Self::Huffman => f.write_str("huffman coding error"),
            Self::InvalidHeader(err) => write!(f, "invalid header: {err}"),
            Self::InvalidPseudoHeader => f.write_str("missplaced pseudo header"),
            Self::InvalidSizeUpdate => f.write_str("invalid size update"),
        }
    }
}

impl From<HeaderError> for HpackError {
    fn from(err: HeaderError) -> Self {
        Self::InvalidHeader(err)
    }
}

impl From<HuffmanError> for HpackError {
    fn from(_: HuffmanError) -> Self {
        Self::Huffman
    }
}

impl From<TryReserveError> for HpackError {
    fn from(_: TryReserveError) -> Self {
        Self::TooMany
    }
}
