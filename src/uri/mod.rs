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
//! [`HttpUri`] is used to represent HTTP specific scheme URI. The difference is that the authority
//! component must be a non-empty host.
//!
//! # Components
//!
//! For each URI components, there is a dedicated struct: [`Scheme`], [`Authority`], [`Host`] and
//! [`Path`]. These struct is provided to be able to build URI from separate parts without
//! concatenating string. For example, HTTP scheme is depends on the connection, authority is from
//! the host header, and path is in request line.
//!
//! # Percent Encoding
//!
//! All API does not Decode or Encode percent encoding by default.
use tcio::bytes::Bytes;

mod matches;
mod parser;
mod impls;
mod error;

#[cfg(test)]
mod test;

/// URI Scheme.
///
/// The scheme component of a URI.
///
/// ```not_rust
///   foo://example.com:8042/over/there?name=ferret
///   \_/
///    |
/// scheme
///    |
///   / \
///   urn:example:animal:ferret:nose
/// ```
///
/// This struct is usually used when building URI [from parts][Uri::from_parts].
///
/// This API follows the [RFC3986](https://www.rfc-editor.org/rfc/rfc3986.html#section-3.1).
///
/// # Example
///
/// To create [`Scheme`] use one of the `Scheme::from_*` method:
///
/// ```
/// use tsue::uri::Scheme;
/// let scheme = Scheme::from_bytes("foo").unwrap();
/// assert_eq!(scheme.as_str(), "foo");
/// ```
#[derive(Clone)]
pub struct Scheme {
    /// is valid ASCII
    value: Bytes,
}

/// HTTP Scheme.
///
/// HTTP/HTTPS scheme.
///
/// This struct is usually used when building HTTP URI [from parts][HttpUri::from_parts].
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct HttpScheme(bool);

/// URI Authority.
///
/// The authority component of a URI.
///
/// ```not_rust
/// foo://username@example.com:8042/over/there?name=ferret
///       \_______________________/
///                   |
///               authority
/// ```
///
/// This struct is usually used when building URI [from parts][Uri::from_parts].
///
/// This API follows the [RFC3986](https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2).
///
/// # Example
///
/// To create [`Authority`] use one of the `Authority::from_*` method:
///
/// ```
/// use tsue::uri::Authority;
/// let authority = Authority::from_bytes("username@example.com:8042").unwrap();
/// assert_eq!(authority.hostname(), "example.com");
/// assert_eq!(authority.port(), Some(8042));
/// assert_eq!(authority.userinfo(), Some("username"));
/// ```
#[derive(Clone)]
pub struct Authority {
    /// is valid ASCII
    value: Bytes,
}

/// URI Host.
///
/// The host component of a URI.
///
/// Host is authority without userinfo.
///
/// ```not_rust
/// foo://username@example.com:8042/over/there?name=ferret
///                \______________/
///                       |
///                     host
/// ```
///
/// This struct is usually used when building HTTP URI [from parts][HttpUri::from_parts].
///
/// # Example
///
/// To create [`Host`] use one of the `Host::from_*` method:
///
/// ```
/// use tsue::uri::Host;
/// let authority = Host::from_bytes("example.com:8042").unwrap();
/// assert_eq!(authority.hostname(), "example.com");
/// assert_eq!(authority.port(), Some(8042));
/// ```
#[derive(Clone)]
pub struct Host {
    /// is valid ASCII
    value: Bytes,
}

/// URI Path.
///
/// The path and query component of a URI.
///
/// ```not_rust
///   foo://example.com:8042/over/there?name=ferret
///                         \_________/ \_________/
///                             |            |
///                            path        query
///        _____________________|__
///       /                        \
///   urn:example:animal:ferret:nose
/// ```
///
/// This struct is usually used when building URI [from parts][Uri::from_parts].
///
/// This API follows the [RFC3986](https://www.rfc-editor.org/rfc/rfc3986.html#section-3.3).
///
/// # Example
///
/// To create [`Path`] use one of the `Path::from_*` method:
///
/// ```
/// use tsue::uri::Path;
/// let path = Path::from_bytes("/over/there?name=ferret").unwrap();
/// assert_eq!(path.path(), "/over/there");
/// assert_eq!(path.query(), Some("name=ferret"));
/// ```
#[derive(Clone)]
pub struct Path {
    /// is valid ASCII
    value: Bytes,
    query: u16,
}

/// URI Generic Syntax.
///
/// This API follows the [RFC3986](https://www.rfc-editor.org/rfc/rfc3986.html).
///
/// # Example
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
/// [`Uri`] is used to represent generic scheme independent URI.
///
/// ```
/// use tsue::uri::Uri;
///
/// let uri = Uri::from_bytes("foo://example.com:8042/over/there?name=ferret").unwrap();
/// assert_eq!(uri.scheme(), "foo");
/// assert_eq!(uri.authority(), Some("example.com:8042"));
/// assert_eq!(uri.path(), "/over/there");
/// assert_eq!(uri.query(), Some("name=ferret"));
///
/// let urn = Uri::from_bytes("urn:example:animal:ferret:nose").unwrap();
/// assert_eq!(urn.scheme(), "urn");
/// assert_eq!(urn.authority(), None);
/// assert_eq!(urn.path(), "example:animal:ferret:nose");
/// ```
#[derive(Debug, Clone)]
pub struct Uri {
    scheme: Scheme,
    authority: Option<Authority>,
    path: Path,
}

/// HTTP URI.
///
/// HTTP/HTTPS Scheme of a URI.
///
/// # Example
///
/// The following is an example HTTP URI and their component parts:
///
/// ```not_rust
///  https://example.com:80/over/there?name=ferret
///  \___/   \____________/\_________/ \_________/
///    |           |          |            |
///  scheme    authority     path        query
/// ```
///
/// [`HttpUri`] used to represent HTTP scheme URI.
///
/// ```rust
/// use tsue::uri::HttpUri;
///
/// let uri = HttpUri::from_bytes("https://example.com:80/over/there?name=ferret").unwrap();
/// assert!(uri.is_https());
/// assert_eq!(uri.authority(), "example.com:80");
/// assert_eq!(uri.path(), "/over/there");
/// assert_eq!(uri.query(), Some("name=ferret"));
/// ```
#[derive(Debug, Clone)]
pub struct HttpUri {
    scheme: HttpScheme,
    host: Host,
    path: Path,
}

pub use error::UriError;
