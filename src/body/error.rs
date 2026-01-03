use std::io;

// ===== Body Error =====

/// A semantic error when reading message body.
#[derive(Debug)]
pub enum BodyError {
    /// Invalid or duplicate Content-Length value.
    InvalidContentLength,
    /// Invalid message body codings.
    InvalidCodings,
    /// Unknown or unsupported `Transfer-Encoding` codings.
    UnknownCodings,
    /// User error where it tries to read empty or exhausted body.
    Exhausted,
    /// User error where body size hint implementation does not match with the chunk length.
    InvalidSizeHint,
    /// Client error where chunked format is invalid.
    InvalidChunked,
    /// Client error where chunked length is too large than the hard limit.
    ChunkTooLarge,
}

impl BodyError {
    const fn message(&self) -> &'static str {
        match self {
            Self::InvalidContentLength => "invalid content length",
            Self::InvalidCodings => "invalid message body codings",
            Self::UnknownCodings => "unknown or unsupported message body codings",
            Self::Exhausted => "message body exhausted",
            Self::InvalidSizeHint => "invalid size hint",
            Self::InvalidChunked => "invalid chunked format",
            Self::ChunkTooLarge => "chunk too large",
        }
    }
}

impl std::error::Error for BodyError { }

impl std::fmt::Display for BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

// ===== Read Body Error =====

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

