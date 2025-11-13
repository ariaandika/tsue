
/// HTTP Parsing error.
#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
}

#[derive(Debug)]
pub enum ParseErrorKind {
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

use ParseErrorKind as Kind;

impl From<Kind> for ParseError {
    #[inline]
    fn from(kind: Kind) -> Self {
        Self { kind }
    }
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Kind::TooLong => f.write_str("request line too long"),
            Kind::InvalidSeparator => f.write_str("invalid separator"),
            Kind::UnknownMethod => f.write_str("unknown method"),
            Kind::InvalidMethod => f.write_str("invalid method"),
            Kind::InvalidTarget => f.write_str("invalid request target"),
            Kind::UnsupportedVersion => f.write_str("unsupported version"),
            Kind::InvalidHeader => f.write_str("invalid header"),
            Kind::MissmatchHost => f.write_str("missmatch host"),
        }
    }
}

impl From<crate::uri::UriError> for ParseError {
    fn from(value: crate::uri::UriError) -> Self {
        use crate::uri::UriError::*;
        match value {
            TooLong => Self::from(ParseErrorKind::TooLong),
            InvalidScheme | InvalidAuthority | InvalidPath => {
                Self::from(ParseErrorKind::InvalidTarget)
            }
        }
    }
}
