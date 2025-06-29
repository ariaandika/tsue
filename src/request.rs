use crate::{body::Body, headers::HeaderMap, method::Method, Version};

mod parser;

/// HTTP Request Parts.
#[derive(Debug)]
pub struct Parts {
    pub method: Method,
    pub headers: HeaderMap,
    pub version: Version,
}

/// HTTP Request.
#[derive(Debug)]
pub struct Request {
    parts: Parts,
    body: Body,
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

    #[inline]
    pub fn into_body(self) -> Body {
        self.body
    }
}


