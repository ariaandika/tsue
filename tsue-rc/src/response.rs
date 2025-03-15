//! http response
use crate::body::ResBody;
use crate::http::{Header, StatusCode, Version, HEADER_SIZE};

pub use writer::{check, write};

mod into_response;
mod writer;

/// an http response parts
#[derive(Default)]
pub struct Parts {
    version: Version,
    status: StatusCode,
    headers: [Header;HEADER_SIZE],
    header_len: usize,
}

impl Parts {
    /// getter for http version
    pub fn version(&self) -> Version {
        self.version
    }

    /// getter for http status
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// getter for http headers
    pub fn headers(&self) -> &[Header] {
        &self.headers[..self.header_len]
    }

    /// insert new header
    pub fn insert_header(&mut self, header: Header) {
        if self.header_len >= HEADER_SIZE {
            return;
        }
        self.headers[self.header_len] = header;
        self.header_len += 1;
    }
}

/// an http response
#[derive(Default)]
pub struct Response {
    parts: Parts,
    body: ResBody,
}

/// construction methods
impl Response {
    /// construct new response with body
    pub fn new(body: ResBody) -> Response {
        Response {
            parts: <_>::default(),
            body,
        }
    }

    /// construct response from parts
    ///
    /// see also [`Response::into_parts`]
    pub fn from_parts(parts: Parts, body: ResBody) -> Response {
        Response { parts, body }
    }

    /// destruct response into parts
    ///
    /// see also [`Response::from_parts`]
    pub fn into_parts(self) -> (Parts, ResBody) {
        (self.parts,self.body)
    }

    /// destruct response into [`ResBody`]
    pub fn into_body(self) -> ResBody {
        self.body
    }
}

/// delegate methods
impl Response {
    /// getter for http version
    pub fn version(&self) -> Version {
        self.parts.version
    }

    /// getter for http status
    pub fn status(&self) -> StatusCode {
        self.parts.status
    }

    /// getter for http headers
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


/// return bad request for error
///
/// implement [`IntoResponse`] with bad request and error message as body
#[derive(Debug)]
pub struct BadRequest<E>(pub E);

mod bad_request {
    use super::*;

    impl<E> BadRequest<E> {
        /// create new [`BadRequest`]
        pub fn new(inner: E) -> Self {
            Self(inner)
        }

        pub fn map<T: From<E>>(self) -> BadRequest<T> {
            BadRequest(self.0.into())
        }
    }

    impl<E> From<E> for BadRequest<E>
    where
        E: std::error::Error,
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

    impl<E> std::error::Error for BadRequest<E> where E: std::error::Error { }

    impl<E> std::fmt::Display for BadRequest<E> where E: std::fmt::Display {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            E::fmt(&self.0, f)
        }
    }
}

