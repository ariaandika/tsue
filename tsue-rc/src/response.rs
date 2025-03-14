//! http response
use crate::body::ResBody;
use crate::http::{Header, StatusCode, Version, HEADER_SIZE};

pub use writer::{check, write};

mod into_response;
mod writer;

#[derive(Default)]
pub struct Parts {
    version: Version,
    status: StatusCode,
    headers: [Header;HEADER_SIZE],
    header_len: usize,
}

impl Parts {
    pub fn version(&self) -> Version {
        self.version
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &[Header] {
        &self.headers[..self.header_len]
    }

    pub fn insert_header(&mut self, header: Header) {
        if self.header_len >= HEADER_SIZE {
            return;
        }
        self.headers[self.header_len] = header;
        self.header_len += 1;
    }
}

#[derive(Default)]
pub struct Response {
    parts: Parts,
    body: ResBody,
}

/// construction methods
impl Response {
    pub fn new(body: ResBody) -> Response {
        Response {
            parts: <_>::default(),
            body,
        }
    }

    pub fn from_parts(parts: Parts, body: ResBody) -> Response {
        Response { parts, body }
    }

    pub fn into_parts(self) -> (Parts, ResBody) {
        (self.parts,self.body)
    }

    pub fn into_body(self) -> ResBody {
        self.body
    }
}

/// delegate methods
impl Response {
    pub fn version(&self) -> Version {
        self.parts.version
    }

    pub fn status(&self) -> StatusCode {
        self.parts.status
    }

    pub fn headers(&self) -> &[Header] {
        self.parts.headers()
    }
}

/// a type that can be converted into response
///
/// this trait is used as request handler return type
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

/// a type that can be converted into response parts
///
/// this trait is used as request handler return type
pub trait IntoResponseParts {
    fn into_response_parts(self, parts: Parts) -> Parts;
}

impl std::fmt::Debug for Parts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parts")
            .field("version", &self.version)
            .field("status", &self.status)
            .field("headers", &self.headers())
            .finish()
    }
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("version", &self.parts.version)
            .field("status", &self.parts.status)
            .field("headers", &self.parts.headers())
            .finish()
    }
}


pub struct BadRequest<E>(E);

mod helpers {
    use super::*;

    impl<E> BadRequest<E> {
        pub fn new(inner: E) -> Self {
            Self(inner)
        }
    }

    impl<E> From<E> for BadRequest<E>
    where
        E: std::fmt::Display
    {
        fn from(value: E) -> Self {
            Self(value)
        }
    }

    impl<E> IntoResponse for BadRequest<E>
    where
        E: std::fmt::Display
    {
        fn into_response(self) -> crate::Response {
            (crate::http::StatusCode::BAD_REQUEST, self.0.to_string()).into_response()
        }
    }
}

