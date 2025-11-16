use crate::h1::parser::ParseError;
use crate::headers::error::HeaderError;

/// HTTP Semantic error.
#[derive(Debug)]
pub enum ProtoError {
    /// Too many headers.
    TooManyHeaders,
    /// Missing host header.
    MissingHost,
    /// Invalid `Connection` header value.
    InvalidConnectionOption,
    /// Invalid Content-Length value.
    InvalidContentLength,
    /// Invalid `Transfer-Encoding` header value.
    InvalidCodings,
    /// Header parsing error.
    HeaderError(HeaderError),
    /// HTTP Parsing error.
    ParseError(ParseError),
}

impl std::error::Error for ProtoError {}

impl std::fmt::Display for ProtoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::TooManyHeaders => f.write_str("too many headers"),
            Self::InvalidContentLength => f.write_str("invalid content length"),
            Self::MissingHost => f.write_str("missing host header"),
            Self::InvalidConnectionOption => f.write_str("invalid connection option"),
            Self::InvalidCodings => f.write_str("invalid message body codings"),
            Self::HeaderError(err) => write!(f, "header error: {err}"),
            Self::ParseError(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl From<HeaderError> for ProtoError {
    #[inline]
    fn from(value: HeaderError) -> Self {
        Self::HeaderError(value)
    }
}

impl From<ParseError> for ProtoError {
    #[inline]
    fn from(value: ParseError) -> Self {
        Self::ParseError(value)
    }
}
