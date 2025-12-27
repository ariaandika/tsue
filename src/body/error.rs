use std::io;

use crate::http::spec::BodyError;

/// Body reading error.
pub struct ReadError {
    kind: Box<Kind>,
}

#[derive(Debug)]
pub enum Kind {
    Body(BodyError),
    Io(io::Error),
}

impl ReadError {
    pub fn kind(&self) -> &Kind {
        &self.kind
    }
}

impl From<BodyError> for ReadError {
    #[inline]
    fn from(v: BodyError) -> Self {
        Self {
            kind: Box::new(Kind::Body(v)),
        }
    }
}

impl From<io::Error> for ReadError {
    #[inline]
    fn from(v: io::Error) -> Self {
        Self {
            kind: Box::new(Kind::Io(v)),
        }
    }
}

impl From<io::ErrorKind> for ReadError {
    #[inline]
    fn from(v: io::ErrorKind) -> Self {
        Self {
            kind: Box::new(Kind::Io(v.into())),
        }
    }
}

impl std::error::Error for ReadError { }

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind.as_ref() {
            Kind::Body(body) => body.fmt(f),
            Kind::Io(error) => error.fmt(f),
        }
    }
}

impl std::fmt::Debug for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ReadError").field(&self.kind).finish()
    }
}

