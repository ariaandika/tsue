//! Uniform Resource Identifier.
//!
//! This API follows the [rfc3986] URI: General Syntax.
//!
//! [rfc3986]: <https://datatracker.ietf.org/doc/html/rfc3986>
use tcio::bytes::Bytes;

mod matches;
mod parser;
mod impls;
mod error;

#[derive(Clone)]
pub struct Scheme {
    /// is valid ASCII
    value: Bytes,
}

#[derive(Clone)]
pub struct Authority {
    /// is valid ASCII
    value: Bytes,
}

#[derive(Clone)]
pub struct Path {
    /// is valid ASCII
    value: Bytes,
    query: u16,
}

/// HTTP [URI][rfc].
///
/// A Uniform Resource Identifier ([URI][rfc]) provides a simple and extensible means for identifying a
/// resource.
///
/// The generic URI syntax consists of a hierarchical sequence of components referred to as the
/// scheme, authority, path, and query.
///
/// The following are two example URIs and their component parts:
///
/// ```not_rust
///   foo://example.com:8042/over/there?name=ferret
///   \_/   \______________/\_________/ \_________/
///    |           |            |            |
/// scheme     authority       path        query
///    |   _____________________|__
///   / \ /                        \
///   urn:example:animal:ferret:nose
/// ```
///
/// [rfc]: <https://datatracker.ietf.org/doc/html/rfc7230#section-2.7>
#[derive(Debug, Clone)]
pub struct Uri {
    scheme: Scheme,
    authority: Option<Authority>,
    path: Path,
}

#[derive(Debug, Clone)]
pub struct HttpUri {
    is_https: bool,
    authority: Authority,
    path: Path,
}

pub use error::UriError;
