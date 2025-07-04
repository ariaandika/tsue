//! HTTP Response
use crate::{
    body::Body,
    headers::HeaderMap,
    http::{Extensions, StatusCode, Version},
};

pub mod write;

/// HTTP Response Parts.
#[derive(Debug, Default)]
pub struct Parts {
    pub version: Version,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub extensions: Extensions,
}

/// HTTP Request.
#[derive(Debug, Default)]
pub struct Response {
    parts: Parts,
    body: Body,
}

impl Response {
    #[inline]
    pub fn parts(&self) -> &Parts {
        &self.parts
    }

    #[inline]
    pub fn parts_mut(&mut self) -> &mut Parts {
        &mut self.parts
    }

    #[inline]
    pub fn body(&self) -> &Body {
        &self.body
    }

    #[inline]
    pub fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }

    #[inline]
    pub fn into_body(self) -> Body {
        self.body
    }
}


