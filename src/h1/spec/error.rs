use crate::{h1::parser::ParseError, headers::error::HeaderError};

/// HTTP Semantic error.
#[derive(Debug)]
pub struct ProtoError {
    kind: ProtoErrorKind,
}

#[derive(Debug)]
pub enum ProtoErrorKind {
    TooManyHeaders,
    InvalidContentLength,
    MissingHost,
    InvalidConnectionOption,
    HeaderError(HeaderError),
    ParseError(ParseError),
}

use ProtoErrorKind as Kind;

impl From<Kind> for ProtoError {
    #[inline]
    fn from(kind: Kind) -> Self {
        Self { kind }
    }
}

impl std::error::Error for ProtoError {}

impl std::fmt::Display for ProtoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::fmt::Display for ProtoErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Kind::TooManyHeaders => f.write_str("too many headers"),
            Kind::InvalidContentLength => f.write_str("invalid content length"),
            Kind::MissingHost => f.write_str("missing host header"),
            Kind::InvalidConnectionOption => f.write_str("invalid connection option"),
            Kind::HeaderError(err) => write!(f, "header error: {err}"),
            Kind::ParseError(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl From<HeaderError> for ProtoError {
    #[inline]
    fn from(value: HeaderError) -> Self {
        Self {
            kind: Kind::HeaderError(value)
        }
    }
}

impl From<ParseError> for ProtoError {
    #[inline]
    fn from(value: ParseError) -> Self {
        Self {
            kind: Kind::ParseError(value),
        }
    }
}
