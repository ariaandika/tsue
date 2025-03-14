//! http request
use crate::http::{Header, Method, Version, HEADER_SIZE};
use crate::{body::Body, bytestr::ByteStr};

pub use parser::{parse, ParseError};

mod from_request;
mod parser;

#[derive(Default)]
pub struct Parts {
    method: Method,
    path: ByteStr,
    version: Version,
    headers: [Header;HEADER_SIZE],
    header_len: usize,
}

impl Parts {
    pub fn method(&self) -> Method {
        self.method
    }

    pub fn path(&self) -> &ByteStr {
        &self.path
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn headers(&self) -> &[Header] {
        &self.headers[..self.header_len]
    }
}

#[derive(Default)]
pub struct Request {
    parts: Parts,
    body: Body,
}

/// construction methods
impl Request {
    pub fn from_parts(parts: Parts, body: Body) -> Request {
        Self { parts, body  }
    }

    pub fn into_parts(self) -> (Parts,Body) {
        (self.parts,self.body)
    }

    pub fn into_body(self) -> Body {
        self.body
    }
}

/// delegate methods
impl Request {
    pub fn method(&self) -> Method {
        self.parts.method
    }

    pub fn path(&self) -> &ByteStr {
        self.parts.path()
    }

    pub fn version(&self) -> Version {
        self.parts.version
    }

    pub fn headers(&self) -> &[Header] {
        self.parts.headers()
    }
}

/// a type that can be constructed from request
///
/// this trait is used as request handler parameters
pub trait FromRequest: Sized {
    type Error;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request(req: Request) -> Self::Future;
}

/// a type that can be constructed from request parts
///
/// this trait is used as request handler parameters
pub trait FromRequestParts: Sized {
    type Error;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request_parts(parts: &mut Parts) -> Self::Future;
}

impl std::fmt::Debug for Parts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parts")
            .field("method", &self.method)
            .field("path", &self.path)
            .field("version", &self.version)
            .field("headers", &self.headers())
            .finish()
    }
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.parts.method)
            .field("path", &self.parts.path)
            .field("version", &self.parts.version)
            .field("headers", &self.parts.headers())
            .finish()
    }
}

