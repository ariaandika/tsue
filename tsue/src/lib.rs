//! HTTP protocol.

#![warn(missing_debug_implementations)]

// NOTE:
// on progress of pulling out modules into other crates
// what should be kept
// - `http` types
// - request and response
// - websocket
// - serialization
// - service ?
// take out
// - from request, into response, routing
// - service ?

pub use tcio::ByteStr;

mod method;
mod path;
mod status;
mod extensions;

pub mod headers;

mod ws;
pub mod body;

// ===== Reexports =====

pub use method::Method;
pub use path::Path;
pub use status::StatusCode;
pub use extensions::Extensions;
pub use headers::{HeaderMap, HeaderName, HeaderValue};

// ===== Types =====

/// HTTP Request parts.
#[derive(Debug)]
pub struct Parts {
    pub method: Method,
    pub path: Path,
    pub headers: HeaderMap,
    pub extensions: Extensions,
}

