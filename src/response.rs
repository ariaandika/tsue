//! HTTP Response
use crate::{
    body::Body,
    headers::HeaderMap,
    http::{Extensions, StatusCode, Version},
};

/// HTTP Response Parts.
#[derive(Debug, Default)]
pub struct Parts {
    pub version: Version,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub extensions: Extensions,
}

/// HTTP Response.
#[derive(Debug, Default)]
pub struct Response {
    parts: Parts,
    body: Body,
}

/// Constructor
impl Response {
    /// Create [`Response`] from [`Parts`] and [`Body`].
    #[inline]
    pub fn from_parts(parts: Parts, body: Body) -> Self {
        Self { parts, body  }
    }
}

impl Response {
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
        /// Returns shared reference to [`Version`].
        version(),
        /// Returns mutable reference to [`Version`].
        version_mut() -> Version;

        /// Returns shared reference to [`Method`].
        status(),
        /// Returns mutable reference to [`Method`].
        status_mut() -> StatusCode;

        /// Returns shared reference to [`HeaderMap`].
        headers(),
        /// Returns mutable reference to [`HeaderMap`].
        headers_mut() -> HeaderMap;
    }

    /// Returns shared reference to [`Body`].
    #[inline]
    pub fn body(&self) -> &Body {
        &self.body
    }

    /// Returns mutable reference to [`Body`].
    #[inline]
    pub fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }
}

/// Destructor
impl Response {
    /// Destruct response into [`Parts`] and [`Body`].
    #[inline]
    pub fn into_parts(self) -> (Parts, Body) {
        (self.parts, self.body)
    }

    /// Destruct response into [`Body`].
    #[inline]
    pub fn into_body(self) -> Body {
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
