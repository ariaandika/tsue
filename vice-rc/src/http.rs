//! http protocol
use crate::bytestring::ByteStr;
use bytes::Bytes;

pub mod status;
pub mod request;
pub mod response;
pub mod from_request;
pub mod into_response;
pub mod service;
pub mod noop;

pub use status::StatusCode;
pub use request::Request;
pub use response::Response;
pub use from_request::{FromRequest, FromRequestParts};
pub use into_response::{IntoResponse, IntoResponseParts};

pub const MAX_HEADER: usize = 32;

#[derive(Default, Debug)]
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

#[derive(Default)]
pub enum Version {
    Http10,
    #[default]
    Http11,
    Http2,
}

impl Version {
    pub const fn as_bytes(&self) -> &'static [u8] {
        match self {
            Version::Http10 => b"HTTP/1.0",
            Version::Http11 => b"HTTP/1.1",
            Version::Http2 =>  b"HTTP/2",
        }
    }
}

#[derive(Clone, Default)]
pub struct Header {
    pub name: ByteStr,
    pub value: Bytes,
}

impl Header {
    pub const fn new_static() -> Header {
        Header {
            name: ByteStr::new(),
            value: Bytes::new(),
        }
    }
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::Http10 => f.write_str("HTTP/1.0"),
            Version::Http11 => f.write_str("HTTP/1.1"),
            Version::Http2 =>  f.write_str("HTTP/2"),
        }
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

