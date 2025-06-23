//! HTTP Types.
mod method;
mod path;
mod status;
mod extensions;

pub mod headers;

pub use headers::{HeaderMap, HeaderName, HeaderValue};
pub use method::Method;
pub use path::PathAndQuery;
pub use status::StatusCode;
pub use extensions::Extensions;

/// The "head" part of HTTP Request or Response.
#[derive(Debug)]
pub struct Parts {
    pub method: Method,
    pub path: PathAndQuery,
    pub headers: HeaderMap,
    pub extensions: Extensions,
}

