use super::{Authority, Path, Scheme};

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
    scheme: Scheme,
    authority: Authority,
    path: Path,
}

impl Uri {
    #[inline]
    pub const fn scheme(&self) -> &str {
        self.scheme.as_str()
    }

    #[inline]
    pub const fn as_scheme(&self) -> &Scheme {
        &self.scheme
    }

    #[inline]
    pub const fn authority(&self) -> &str {
        self.authority.as_str()
    }

    #[inline]
    pub const fn as_authority(&self) -> &Authority {
        &self.authority
    }

    #[inline]
    pub const fn path(&self) -> &str {
        self.path.path()
    }

    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.path.as_str()
    }
}
