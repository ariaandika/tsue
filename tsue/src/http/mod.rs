//! HTTP Types.
mod method;
mod path;

pub mod headers;

pub use method::Method;
pub use path::PathAndQuery;
pub use headers::{
    HeaderMap, HeaderName, HeaderValue,
};

/// The "head" part of HTTP Request or Response.
#[derive(Debug)]
pub struct Parts {
    pub method: Method,
    pub path: PathAndQuery,
    pub headers: HeaderMap,
}

