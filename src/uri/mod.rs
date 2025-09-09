//! Uniform Resource Identifier.
//!
//! This API follows the [rfc3986] URI: General Syntax.
//!
//! [rfc3986]: <https://datatracker.ietf.org/doc/html/rfc3986>
mod simd;
mod scheme;
mod authority;
mod path;
mod error;

#[allow(clippy::module_inception)]
mod uri;

pub use scheme::Scheme;
pub use authority::Authority;
pub use path::Path;
pub use uri::Uri;
pub use error::UriError;

