use std::slice::from_raw_parts;

use super::{Authority, Path, Scheme, Uri, simd};

impl Scheme {
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

impl Authority {
    const fn find_at(&self) -> Option<tcio::bytes::Cursor<'_>> {
        simd::find_at!(
            self.value;
            match {
                Some(cursor) => Some(cursor),
                None => None,
            }
        )
    }

    const fn find_col(&self) -> Option<tcio::bytes::Cursor<'_>> {
        let mut cursor = simd::find_at!(
            self.value;
            match {
                Some(cursor) => cursor,
                None => self.value.cursor(),
            }
        );
        simd::find_col! {
            match {
                Some(cursor) => Some(cursor),
                None => None,
            }
        }
    }

    /// Returns the authority host.
    #[inline]
    pub const fn host(&self) -> Option<&str> {
        match self.find_at() {
            Some(cursor) => unsafe { Some(str::from_utf8_unchecked(cursor.as_slice())) },
            None => None,
        }
    }

    /// Returns the authority hostname.
    #[inline]
    pub const fn hostname(&self) -> &str {
        let hostname = match self.find_col() {
            Some(cursor) => cursor.advanced_slice(),
            None => self.value.as_slice(),
        };
        unsafe { str::from_utf8_unchecked(hostname) }
    }

    /// Returns the authority port.
    #[inline]
    pub const fn port(&self) -> Option<u16> {
        match self.find_col() {
            Some(mut cursor) => {
                // with port validation in constructor, should we do unsafe calculation ?
                cursor.advance(1);
                match tcio::atou(cursor.as_slice()) {
                    Some(ok) => Some(ok as u16),
                    None => None,
                }
            }
            None => None,
        }
    }

    /// Returns the authority userinfo.
    #[inline]
    pub const fn userinfo(&self) -> Option<&str> {
        match self.find_at() {
            Some(mut cursor) => unsafe {
                cursor.step_back(1);
                Some(str::from_utf8_unchecked(cursor.as_slice()))
            },
            None => None,
        }
    }

    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

impl Path {
    /// Returns the path as `str`, e.g: `/over/there`.
    #[inline]
    pub const fn path(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII, `query` is less than or equal to `value`
        // length
        unsafe {
            str::from_utf8_unchecked(from_raw_parts(self.value.as_ptr(), self.query as usize))
        }
    }

    /// Returns the query as `str`, e.g: `name=joe&query=4`.
    #[inline]
    pub const fn query(&self) -> Option<&str> {
        if self.query as usize == self.value.len() {
            None
        } else {
            // SAFETY: precondition `value` is valid ASCII
            unsafe {
                let query = self.query as usize;
                Some(str::from_utf8_unchecked(from_raw_parts(
                    self.value.as_ptr().add(query.unchecked_add(1)),
                    self.value.len().unchecked_sub(query),
                )))
            }
        }
    }

    /// Returns the path and query as `str`, e.g: `/over/there?name=joe&query=4`.
    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.as_str()
    }

    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

impl Uri {
    #[inline]
    pub const fn from_parts(scheme: Scheme, authority: Option<Authority>, path: Path) -> Self {
        Self {
            scheme,
            authority,
            path,
        }
    }

    #[inline]
    pub const fn scheme(&self) -> &str {
        self.scheme.as_str()
    }

    #[inline]
    pub const fn as_scheme(&self) -> &Scheme {
        &self.scheme
    }

    #[inline]
    pub const fn authority(&self) -> Option<&str> {
        match &self.authority {
            Some(auth) => Some(auth.as_str()),
            None => None,
        }
    }

    #[inline]
    pub const fn as_authority(&self) -> Option<&Authority> {
        self.authority.as_ref()
    }

    #[inline]
    pub const fn path(&self) -> &str {
        self.path.path()
    }

    #[inline]
    pub const fn query(&self) -> Option<&str> {
        self.path.query()
    }

    #[inline]
    pub const fn path_and_query(&self) -> &str {
        self.path.as_str()
    }
}

// ===== Formatting =====

macro_rules! delegate_fmt {
    ($($ty:ty),*) => {
        $(
            impl std::fmt::Debug for $ty {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.as_str().fmt(f)
                }
            }

            impl std::fmt::Display for $ty {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.as_str().fmt(f)
                }
            }
        )*
    };
    () => {}
}

delegate_fmt! {
    Scheme,
    Authority,
    Path
}
