use crate::h2::hpack::error::HpackError;
use crate::headers::error::HeaderError;

// ===== Error Code =====

/// HTTP/2 Error Codes.
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorCode {
    /// The associated condition is not a result of an error.
    ///
    /// For example, a GOAWAY might include this code to indicate graceful shutdown of a
    /// connection.
    NoError = 0x00,
    /// The endpoint detected an unspecific protocol error.
    ///
    /// This error is for use when a more specific error code is not available.
    ProtocolError = 0x01,
    /// The endpoint encountered an unexpected internal error.
    InternalError = 0x02,
    /// The endpoint detected that its peer violated the flow-control protocol.
    FlowControlError = 0x03,
    /// The endpoint sent a SETTINGS frame but did not receive a response in a timely manner.
    SettingsTimeout = 0x04,
    /// The endpoint received a frame after a stream was half-closed.
    StreamClosed = 0x05,
    /// The endpoint received a frame with an invalid size.
    FrameSizeError = 0x06,
    /// The endpoint refused the stream prior to performing any application processing.
    RefusedStream = 0x07,
    /// The endpoint uses this error code to indicate that the stream is no longer needed.
    Cancel = 0x08,
    /// The endpoint is unable to maintain the field section compression context for the
    /// connection.
    CompressionError = 0x09,
    /// The connection established in response to a CONNECT request was reset or abnormally closed.
    ConnectError = 0x0a,
    /// The endpoint detected that its peer is exhibiting a behavior that might be generating
    /// excessive load.
    EnhanceYourCalm = 0x0b,
    /// The underlying transport has properties that do not meet minimum security requirements.
    InadequateSecurity = 0x0c,
    /// The endpoint requires that HTTP/1.1 be used instead of HTTP/2.
    Http11Required = 0x0d,
}

// ===== Handshake Error =====

/// An error that can occur during handshake.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandshakeError {
    /// Invalid preface bytes.
    InvalidPreface,
    /// Expected SETTINGS frame.
    ExpectedSettings,
}

impl std::error::Error for HandshakeError {}
impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPreface => f.write_str("invalid preface bytes"),
            Self::ExpectedSettings => f.write_str("expected SETTINGS frame"),
        }
    }
}

// ===== Connection Error =====

/// A fatal error that can occur in connection causing shutdown.
#[derive(Clone, Debug)]
pub enum ConnectionError {
    /// Unexpected frame.
    UnexpectedFrame,
    /// Unknown setting identifier.
    UnknownSetting,
    /// Invalid stream id.
    InvalidStreamId,
    /// Excessive frame payload length.
    ExcessiveFrame,
    /// Excessive headers list length.
    ExcessiveHeaders,
    /// Header error.
    Header(HeaderError),
    /// Hpack error.
    Hpack(HpackError),
}

impl std::error::Error for ConnectionError {}
impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedFrame  => f.write_str("unexpected frame"),
            Self::UnknownSetting => f.write_str("unknown setting identifier"),
            Self::InvalidStreamId => f.write_str("invalid stream id"),
            Self::ExcessiveFrame => f.write_str("excessive frame length"),
            Self::ExcessiveHeaders => f.write_str("excessive headers list"),
            Self::Header(err) => err.fmt(f),
            Self::Hpack(err) => err.fmt(f),
        }
    }
}

impl From<HeaderError> for ConnectionError {
    fn from(v: HeaderError) -> Self {
        Self::Header(v)
    }
}

impl From<HpackError> for ConnectionError {
    fn from(v: HpackError) -> Self {
        Self::Hpack(v)
    }
}
