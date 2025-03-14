//! http request
use crate::http::{Header, Method, Version, HEADER_SIZE};
use crate::IntoResponse;
use crate::{body::Body, bytestr::ByteStr};

pub use parser::{parse, ParseError};

mod from_request;
mod parser;

/// an http request parts
#[derive(Default)]
pub struct Parts {
    method: Method,
    path: ByteStr,
    version: Version,
    headers: [Header;HEADER_SIZE],
    header_len: usize,
}

impl Parts {
    /// getter for http method
    pub fn method(&self) -> Method {
        self.method
    }

    /// getter for http path
    pub fn path(&self) -> &ByteStr {
        &self.path
    }

    /// getter for http version
    pub fn version(&self) -> Version {
        self.version
    }

    /// getter for http headers
    pub fn headers(&self) -> &[Header] {
        &self.headers[..self.header_len]
    }
}

/// an http request
#[derive(Default)]
pub struct Request {
    parts: Parts,
    body: Body,
}

/// construction methods
impl Request {
    /// construct request from parts
    ///
    /// see also [`Request::into_parts`]
    pub fn from_parts(parts: Parts, body: Body) -> Request {
        Self { parts, body  }
    }

    /// destruct request into parts
    ///
    /// see also [`Request::from_parts`]
    pub fn into_parts(self) -> (Parts,Body) {
        (self.parts,self.body)
    }

    /// destruct request into [`Body`]
    pub fn into_body(self) -> Body {
        self.body
    }
}

/// delegate methods
impl Request {
    /// getter for http method
    pub fn method(&self) -> Method {
        self.parts.method
    }

    /// getter for http path
    pub fn path(&self) -> &ByteStr {
        self.parts.path()
    }

    /// getter for http version
    pub fn version(&self) -> Version {
        self.parts.version
    }

    /// getter for http headers
    pub fn headers(&self) -> &[Header] {
        self.parts.headers()
    }
}

/// a type that can be constructed from request
///
/// this trait is used as request handler parameters
pub trait FromRequest: Sized {
    type Error: IntoResponse;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request(req: Request) -> Self::Future;
}

/// a type that can be constructed from request parts
///
/// this trait is used as request handler parameters
pub trait FromRequestParts: Sized {
    type Error: IntoResponse;
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

