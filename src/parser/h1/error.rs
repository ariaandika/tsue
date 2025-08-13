
/// HTTP Parsing error.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Self { kind }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    /// Request line is too short.
    TooShort,
    /// Request line is too long.
    TooLong,
    /// Request line have invalid separator
    InvalidSeparator,
    /// HTTP Method is unknown.
    UnknownMethod,
    /// HTTP Version is unsupported.
    UnsupportedVersion,
    /// Headers exceed configured maximum count.
    TooManyHeaders,
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::UnknownMethod => f.write_str("unknown method"),
            ErrorKind::TooShort => f.write_str("request line too short"),
            ErrorKind::TooLong => f.write_str("request line too long"),
            ErrorKind::UnsupportedVersion => f.write_str("unsupported HTTP version"),
            ErrorKind::InvalidSeparator => f.write_str("invalid separator"),
            ErrorKind::TooManyHeaders => f.write_str("received headers count exceeded the configured maximum"),
        }
    }
}
