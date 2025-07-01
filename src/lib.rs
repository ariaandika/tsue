//! HTTP protocol.
#![warn(missing_debug_implementations)]

// NOTE:
// on progress of pulling out modules into other crates
// - serialization
// - service
// take out
// - from request, into response, routing

pub use tcio::ByteStr;

mod method;
mod path;
mod status;
mod version;
mod extensions;

pub mod headers;

pub mod request;
pub mod response;
mod ws;
pub mod body;

pub mod service;
pub mod rt;

// ===== Reexports =====

pub use method::{Method, UnknownMethod};
pub use path::Path;
pub use version::Version;
pub use status::StatusCode;
pub use extensions::Extensions;
pub use headers::{HeaderMap, HeaderName, HeaderValue};

