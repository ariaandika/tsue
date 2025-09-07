
/// HTTP Parsing error.
#[derive(Debug)]
pub struct HttpError {
    kind: ErrorKind,
}

impl From<ErrorKind> for HttpError {
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

impl std::error::Error for HttpError {}
impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.kind {
            ErrorKind::TooShort => f.write_str("request line too short"),
            ErrorKind::TooLong => f.write_str("request line too long"),
            ErrorKind::InvalidSeparator => f.write_str("invalid separator"),
            ErrorKind::UnknownMethod => f.write_str("unknown method"),
            ErrorKind::InvalidMethod => f.write_str("invalid method"),
            ErrorKind::InvalidTarget => f.write_str("invalid request target"),
            ErrorKind::UnsupportedVersion => f.write_str("unsupported version"),
            ErrorKind::InvalidHeader => f.write_str("invalid header"),
            ErrorKind::MissmatchHost => f.write_str("missmatch host"),
        }
    }
}

impl From<crate::uri::UriError> for HttpError {
    fn from(value: crate::uri::UriError) -> Self {
        match value {
            crate::uri::UriError::Incomplete => Self::from(ErrorKind::TooShort),
            crate::uri::UriError::TooLong => Self::from(ErrorKind::TooLong),
            crate::uri::UriError::Char => Self::from(ErrorKind::InvalidTarget),
        }
    }
}
