//! Uniform Resource Identifier ([RFC3986])
//!
//! [RFC3986]: <https://www.rfc-editor.org/rfc/rfc3986.html>
//!
//! # Generic Syntax
//!
//! The following are two example URIs and their component parts:
//!
//! ```not_rust
//!   foo://example.com:8042/over/there?name=ferret
//!   \_/   \______________/\_________/ \_________/
//!    |           |            |            |
//! scheme     authority       path        query
//!    |   _____________________|__
//!   / \ /                        \
//!   urn:example:animal:ferret:nose
//! ```
//!
//! [`Uri`] is used to represent generic scheme independent URI.
//!
//! ```
//! use tsue::uri::Uri;
//!
//! let uri = Uri::from_bytes("foo://example.com:8042/over/there?name=ferret").unwrap();
//! assert_eq!(uri.scheme(), "foo");
//! assert_eq!(uri.authority(), Some("example.com:8042"));
//! assert_eq!(uri.path(), "/over/there");
//! assert_eq!(uri.query(), Some("name=ferret"));
//!
//! let urn = Uri::from_bytes("urn:example:animal:ferret:nose").unwrap();
//! assert_eq!(urn.scheme(), "urn");
//! assert_eq!(urn.authority(), None);
//! assert_eq!(urn.path(), "example:animal:ferret:nose");
//! ```
//!
//! # HTTP URI
//!
//! [`HttpUri`] is used to represent HTTP specific scheme URI. The difference is that the authority
//! component must be a non-empty host and userinfo are not allowed.
//!
//! # Components
//!
//! There is a dedicated struct for some URI components: [`Scheme`], [`Authority`], [`Host`] and
//! [`Path`].
//!
//! # Percent Encoding
//!
//! All API does not Decode or Encode percent encoding.
mod matches;
mod scheme;
mod authority;
mod path;
mod http;
mod error;

#[allow(clippy::module_inception)]
mod uri;

pub use scheme::Scheme;
pub use authority::{Authority, Host};
pub use path::Path;
pub use uri::Uri;
pub use http::{HttpScheme, HttpUri};
pub use error::UriError;

#[cfg(test)]
mod test;
