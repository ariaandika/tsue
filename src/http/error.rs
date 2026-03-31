use crate::headers::error::HeaderError;

// ===== Parsing Error =====W

/// HTTP Parsing error.
#[derive(Debug)]
pub enum ParseError {
    /// Excessive bytes length.
    ExcessiveBytes,
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
            Self::ExcessiveBytes => f.write_str("request line too long"),
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
        use UriError as U;
        match value {
            U::ExcessiveBytes => Self::ExcessiveBytes,
            U::InvalidScheme
            | U::InvalidAuthority
            | U::InvalidPath
            | U::InvalidHost
            | U::InvalidPort => Self::InvalidTarget,
        }
    }
}

// ===== UriError =====

/// An error that can occur during URI validation.
#[derive(Debug, Clone)]
pub enum UriError {
    /// Excessive bytes length.
    ExcessiveBytes,
    /// Invalid scheme.
    InvalidScheme,
    /// Invalid authority.
    InvalidAuthority,
    /// Invalid host.
    InvalidHost,
    /// Invalid port.
    InvalidPort,
    /// Invalid path.
    InvalidPath,
}

// ===== Error =====

macro_rules! gen_error {
    ($($variant:pat => $msg:literal),* $(,)?) => {
        impl UriError {
            pub(crate) const fn panic_const(&self) -> ! {
                use UriError::*;
                match self {
                    $($variant => panic!($msg),)*
                }
            }
        }

        impl std::fmt::Display for UriError {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                use UriError::*;
                match self {
                    $($variant => $msg.fmt(f),)*
                }
            }
        }
    };
}

gen_error! {
    ExcessiveBytes => "excessive bytes length",
    InvalidScheme => "invalid scheme",
    InvalidAuthority => "invalid authority",
    InvalidHost => "invalid host",
    InvalidPort => "invalid port",
    InvalidPath => "invalid path",
}

impl std::error::Error for UriError { }

// ===== Protocol Error =====

/// HTTP Semantic error.
#[derive(Debug)]
pub enum ProtoError {
    /// Excessive headers count.
    ExcessiveHeaders,
    /// Missing, duplicate, or invalid host header.
    InvalidHost,
    /// Missing, duplicate, or invalid representation metadata.
    InvalidRepresentation,
    /// Invalid `Connection` header value.
    InvalidConnectionOption,
    /// Invalid or duplicate Content-Length value.
    InvalidContentLength,
    /// Invalid message body codings.
    InvalidCodings,
    /// Unsupported transfer codings.
    UnsupportedCodings,
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
            Self::ExcessiveHeaders => f.write_str("excessive headers count"),
            Self::InvalidHost => f.write_str("invalid host"),
            Self::InvalidRepresentation => f.write_str("invalid representation metadata"),
            Self::InvalidContentLength => f.write_str("invalid content length"),
            Self::InvalidConnectionOption => f.write_str("invalid connection option"),
            Self::InvalidCodings => f.write_str("invalid message body codings"),
            Self::UnsupportedCodings => f.write_str("unsupported transfer codings"),
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

impl From<crate::headers::error::TryReserveError> for ProtoError {
    #[inline]
    fn from(_: crate::headers::error::TryReserveError) -> Self {
        Self::ExcessiveHeaders
    }
}

// ===== User Error =====

/// User error.
#[derive(Debug)]
pub enum UserError {
    ExcessiveContent,
    UnreadRequestContent,
}

impl std::error::Error for UserError {}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ExcessiveContent => f.write_str("user content is larger than given size hint"),
            Self::UnreadRequestContent => f.write_str("user did not drain the request content"),
        }
    }
}

