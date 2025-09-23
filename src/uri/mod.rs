//! Uniform Resource Identifier ([RFC3986])
//!
//! [RFC3986]: <https://datatracker.ietf.org/doc/html/rfc3986>
//!
//! # Generic Syntax
//!
//! [`Uri`] used to represent generic scheme independent URI.
//!
//! # Percent Encoding
//!
//! All API here does not Decode or Encode percent encoding by default. Use `decode`/`encode`
//! method on correspnding API to decode or encode percent encoding respectively.
use tcio::bytes::Bytes;

mod matches;
mod parser;
mod impls;
mod error;

/// URI Scheme.
#[derive(Clone)]
pub struct Scheme {
    /// is valid ASCII
    value: Bytes,
}

/// URI Authority.
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

/// URI Generic Syntax ([RFC3986])
///
/// [RFC3986]: <https://datatracker.ietf.org/doc/html/rfc3986>
///
/// # Syntax Component
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
