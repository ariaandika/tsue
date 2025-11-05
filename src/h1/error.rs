use std::fmt;

use crate::{h1::parser::H1ParseError, headers::error::HeaderError};

#[derive(Debug)]
pub struct H1Error {
    kind: H1ErrorKind,
}

#[derive(Debug)]
pub enum H1ErrorKind {
    TooManyHeaders,
    InvalidContentLength,
    MissingHost,
    HeaderError(HeaderError),
    ParseError(H1ParseError),
}

use H1ErrorKind as Kind;

impl From<Kind> for H1Error {
    fn from(kind: Kind) -> Self {
        Self { kind }
    }
}

impl std::error::Error for H1Error { }

impl fmt::Display for H1Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl fmt::Display for H1ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::TooManyHeaders => f.write_str("too many headers"),
            Kind::InvalidContentLength => f.write_str("invalid content length"),
            Kind::MissingHost => f.write_str("missing host header"),
            Kind::HeaderError(err) => write!(f, "header error: {err}"),
            Kind::ParseError(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl From<HeaderError> for H1Error {
    fn from(v: HeaderError) -> Self {
        Self {
            kind: Kind::HeaderError(v),
        }
    }
}

impl From<H1ParseError> for H1Error {
    fn from(v: H1ParseError) -> Self {
        Self {
            kind: Kind::ParseError(v),
        }
    }
}

