use crate::headers::error::{HeaderError, HeaderNameError, HeaderValueError};
use crate::uri::UriError;

// ===== Parsing Error =====W

/// HTTP Parsing error.
#[derive(Debug)]
pub enum ParseError {
    /// Request line is too long.
    TooLong,
    /// Request line have invalid separator
    InvalidSeparator,
    /// Unknown Method.
    UnknownMethod,
    /// Invalid character in method.
    InvalidMethod,
    /// Invalid character in request target.
    InvalidTarget,
    /// Unsupported version.
    UnsupportedVersion,
    /// Invalid header name.
    InvalidHeader,
    /// Host header and absolute/authority request target is missmatch.
    MissmatchHost,
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::TooLong => f.write_str("request line too long"),
            Self::InvalidSeparator => f.write_str("invalid separator"),
            Self::UnknownMethod => f.write_str("unknown method"),
            Self::InvalidMethod => f.write_str("invalid method"),
            Self::InvalidTarget => f.write_str("invalid request target"),
            Self::UnsupportedVersion => f.write_str("unsupported version"),
            Self::InvalidHeader => f.write_str("invalid header"),
            Self::MissmatchHost => f.write_str("missmatch host"),
        }
    }
}

impl From<UriError> for ParseError {
    fn from(value: UriError) -> Self {
        use crate::uri::UriError as Error;
        match value {
            Error::TooLong => Self::TooLong,
            Error::InvalidScheme | Error::InvalidAuthority | Error::InvalidPath => {
                Self::InvalidTarget
            }
        }
    }
}

// ===== Protocol Error =====

/// HTTP Semantic error.
#[derive(Debug)]
pub enum ProtoError {
    /// Too many headers.
    TooManyHeaders,
    /// Missing host header.
    MissingHost,
    /// Invalid `Connection` header value.
    InvalidConnectionOption,
    /// Invalid or duplicate Content-Length value.
    InvalidContentLength,
    /// Invalid message body codings.
    InvalidCodings,
    /// Unknown or unsupported `Transfer-Encoding` codings.
    UnknownCodings,
    /// Too many `Transfer-Encoding` values.
    TooManyEncodings,
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
            Self::UnknownCodings => f.write_str("unknown or unsupported message body codings"),
            Self::TooManyEncodings => f.write_str("too many chunked encodings"),
            Self::HeaderError(err) => write!(f, "header error: {err}"),
            Self::ParseError(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl From<UriError> for ProtoError {
    #[inline]
    fn from(value: UriError) -> Self {
        Self::ParseError(value.into())
    }
}

impl From<ParseError> for ProtoError {
    #[inline]
    fn from(value: ParseError) -> Self {
        Self::ParseError(value)
    }
}

impl From<HeaderError> for ProtoError {
    #[inline]
    fn from(value: HeaderError) -> Self {
        Self::HeaderError(value)
    }
}

impl From<HeaderNameError> for ProtoError {
    #[inline]
    fn from(value: HeaderNameError) -> Self {
        Self::HeaderError(value.into())
    }
}

impl From<HeaderValueError> for ProtoError {
    #[inline]
    fn from(value: HeaderValueError) -> Self {
        Self::HeaderError(value.into())
    }
}
