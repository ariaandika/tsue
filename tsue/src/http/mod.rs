//! HTTP Types.
mod method;
mod path;
mod status;
mod extensions;

pub mod headers;

pub use method::Method;
pub use path::Path;
pub use status::StatusCode;
pub use extensions::Extensions;
pub use headers::{HeaderMap, HeaderName, HeaderValue};

/// HTTP Request parts.
#[derive(Debug)]
pub struct Parts {
    pub method: Method,
    pub path: Path,
    pub headers: HeaderMap,
    pub extensions: Extensions,
}

