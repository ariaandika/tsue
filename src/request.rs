//! HTTP Request
use crate::{
    headers::HeaderMap,
    http::{Extensions, Method, Version},
    uri::{Host, HttpScheme, HttpUri, Path},
};

/// HTTP Request Parts.
#[derive(Debug, Clone)]
pub struct Parts {
    pub method: Method,
    pub uri: HttpUri,
    pub version: Version,
    pub headers: HeaderMap,
    pub extensions: Extensions,
}

impl Default for Parts {
    fn default() -> Self {
        Self {
            method: <_>::default(),
            uri: HttpUri::from_parts(
                HttpScheme::HTTP,
                Host::from_static(b""),
                Path::from_static(b"/"),
            ),
            version: <_>::default(),
            headers: <_>::default(),
            extensions: <_>::default(),
        }
    }
}

/// HTTP Request.
#[derive(Debug, Default)]
pub struct Request<T> {
    parts: Parts,
    body: T,
}

/// Constructor
impl<T> Request<T> {
    /// Create [`Request`] from [`Parts`] and body.
    #[inline]
    pub fn from_parts(parts: Parts, body: T) -> Self {
        Self { parts, body  }
    }
}

impl<T> Request<T> {
    /// Returns shared reference to [`Parts`].
    #[inline]
    pub fn parts(&self) -> &Parts {
        &self.parts
    }

    /// Returns mutable reference to [`Parts`].
    #[inline]
    pub fn parts_mut(&mut self) -> &mut Parts {
        &mut self.parts
    }

    delegate! {
        /// Returns shared reference to [`Method`].
        method(),
        /// Returns mutable reference to [`Method`].
        method_mut() -> Method;

        /// Returns shared reference to [`HttpUri`].
        uri(),
        /// Returns mutable reference to [`HttpUri`].
        uri_mut() -> HttpUri;

        /// Returns shared reference to [`Version`].
        version(),
        /// Returns mutable reference to [`Version`].
        version_mut() -> Version;

        /// Returns shared reference to [`HeaderMap`].
        headers(),
        /// Returns mutable reference to [`HeaderMap`].
        headers_mut() -> HeaderMap;
    }

    /// Returns shared reference to body.
    #[inline]
    pub fn body(&self) -> &T {
        &self.body
    }

    /// Returns mutable reference to body.
    #[inline]
    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }
}

/// Destructor
impl<T> Request<T> {
    /// Destruct request into [`Parts`] and body.
    #[inline]
    pub fn into_parts(self) -> (Parts, T) {
        (self.parts, self.body)
    }

    /// Destruct request into [`Body`].
    #[inline]
    pub fn into_body(self) -> T {
        self.body
    }
}

// ===== Macros =====

macro_rules! delegate {
    (@CORE
        $(#[$rdoc:meta])*
        $mref:ident(),
        $(#[$mdoc:meta])*
        $mmut:ident() -> $ty:ty
    ) => {
        $(#[$rdoc])*
        #[inline]
        pub fn $mref(&self) -> &$ty {
            &self.parts.$mref
        }

        $(#[$mdoc])*
        #[inline]
        pub fn $mmut(&mut self) -> &mut $ty {
            &mut self.parts.$mref
        }
    };
    (
        $(
            $(#[$rdoc:meta])*
            $mref:ident(),
            $(#[$mdoc:meta])*
            $mmut:ident() -> $ty:ty;
        )*
    ) => {
        $(
            delegate! {
                @CORE
                $(#[$rdoc])*
                $mref(),
                $(#[$mdoc])*
                $mmut() -> $ty
            }
        )*
    };
}

use {delegate};
