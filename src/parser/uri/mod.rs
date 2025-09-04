mod simd;

#[allow(clippy::module_inception)]
mod uri;
mod path;
mod parser;
mod error;

use tcio::bytes::ByteStr;

pub use error::UriError;

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
//
// Internally:
//
// ```
//   foo://example.com:8042/over/there?name=ferret
//     _/          ________|___       \_____
//    /           /            \            \
// scheme     authority       path        query
//
//   foo:/over/there
//     _/\___       \_____
//    /      \            \
// scheme   path        query
//
//   /over/there
//   \___       \_____
//       \            \
//      path        query
//
//   example.com
//      ________|______
//     /          \    \
// authority    path  query
// ```
#[derive(Debug, Clone)]
pub struct Uri {
    value: ByteStr,
    scheme: u16,
    authority: u16,
    path: u16,
    query: u16,
}

/// Path only URI.
#[derive(Debug, Clone)]
pub struct Path {
    value: ByteStr,
    query: u16,
}

#[cfg(test)]
mod test;
