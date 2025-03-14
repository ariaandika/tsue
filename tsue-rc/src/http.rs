//! http protocol
use crate::bytestr::ByteStr;
use bytes::Bytes;

mod status;

pub use status::StatusCode;

/// default header array size
pub const HEADER_SIZE: usize = 32;

/// an http method
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Method {
    #[default]
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
    CONNECT,
}

/// an http version
#[derive(Clone, Copy, Default)]
pub enum Version {
    Http10,
    #[default]
    Http11,
    Http2,
}

impl Version {
    /// version as bytes (e.g: `b"HTTP/1.1"`)
    pub const fn as_bytes(&self) -> &'static [u8] {
        match self {
            Version::Http10 => b"HTTP/1.0",
            Version::Http11 => b"HTTP/1.1",
            Version::Http2 =>  b"HTTP/2",
        }
    }
}

/// an http header
#[derive(Clone, Default)]
pub struct Header {
    pub name: ByteStr,
    pub value: Bytes,
}

impl Header {
    /// create empty header
    pub const fn new_static() -> Header {
        Header {
            name: ByteStr::new(),
            value: Bytes::new(),
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::GET => f.write_str("GET"),
            Method::POST => f.write_str("POST"),
            Method::PUT => f.write_str("PUT"),
            Method::PATCH => f.write_str("PATCH"),
            Method::DELETE => f.write_str("DELETE"),
            Method::HEAD => f.write_str("HEAD"),
            Method::CONNECT => f.write_str("CONNECT"),
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::Http10 => f.write_str("HTTP/1.0"),
            Version::Http11 => f.write_str("HTTP/1.1"),
            Version::Http2 =>  f.write_str("HTTP/2"),
        }
    }
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Debug for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Header")
            .field("name", &self.name)
            .field("value", &std::str::from_utf8(&self.value).unwrap_or("<bytes>"))
            .finish()
    }
}

