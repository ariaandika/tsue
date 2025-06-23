//! HTTP Types.
mod method;
mod path;
mod status;

pub mod headers;

pub use headers::{HeaderMap, HeaderName, HeaderValue};
pub use method::Method;
pub use path::PathAndQuery;
pub use status::StatusCode;

/// The "head" part of HTTP Request or Response.
#[derive(Debug)]
pub struct Parts {
    pub method: Method,
    pub path: PathAndQuery,
    pub headers: HeaderMap,
}

