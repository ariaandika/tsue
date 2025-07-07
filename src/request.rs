//! HTTP Request
use crate::{
    body::Body,
    headers::HeaderMap,
    http::{Extensions, Method, Uri, Version},
};

pub mod parser;

/// HTTP Request Parts.
#[derive(Debug)]
pub struct Parts {
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub headers: HeaderMap,
    pub extensions: Extensions,
}

/// HTTP Request.
#[derive(Debug)]
pub struct Request {
    parts: Parts,
    body: Body,
}

/// Constructor
impl Request {
    /// Create [`Request`] from [`Parts`] and [`Body`].
    #[inline]
    pub fn from_parts(parts: Parts, body: Body) -> Self {
        Self { parts, body  }
    }
}

impl Request {
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
}

/// Destructor
impl Request {
    #[inline]
    pub fn into_parts(self) -> (Parts, Body) {
        (self.parts, self.body)
    }

    #[inline]
    pub fn into_body(self) -> Body {
        self.body
    }
}


