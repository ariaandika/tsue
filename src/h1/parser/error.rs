
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

impl From<crate::uri::UriError> for ParseError {
    fn from(value: crate::uri::UriError) -> Self {
        use crate::uri::UriError as Error;
        match value {
            Error::TooLong => Self::TooLong,
            Error::InvalidScheme | Error::InvalidAuthority | Error::InvalidPath => {
                Self::InvalidTarget
            }
        }
    }
}
