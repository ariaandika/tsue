use std::slice::from_raw_parts;

use super::{Authority, Path, Scheme, Uri};

impl Scheme {
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: precondition `value` is valid ASCII
        unsafe { str::from_utf8_unchecked(self.value.as_slice()) }
    }
}

impl Authority {
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
    pub const fn from_parts(scheme: Scheme, authority: Authority, path: Path) -> Self {
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
