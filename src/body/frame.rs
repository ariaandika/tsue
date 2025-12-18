use crate::headers::HeaderMap;

/// HTTP Data frame.
#[derive(Debug)]
pub struct Frame<T> {
    repr: Repr<T>,
}

#[derive(Debug)]
enum Repr<T> {
    Data(T),
    Trailers(HeaderMap),
}

impl<T> Frame<T> {
    /// Create new DATA frame.
    #[inline]
    pub const fn data(data: T) -> Self {
        Self { repr: Repr::Data(data) }
    }

    /// Create new trailers frame.
    #[inline]
    pub const fn trailers(trailers: HeaderMap) -> Self {
        Self { repr: Repr::Trailers(trailers) }
    }

    /// Returns `true` if this is a DATA frame.
    #[inline]
    pub const fn is_data(&self) -> bool {
        matches!(self.repr, Repr::Data(_))
    }

    /// Returns `true` if this is a trailers frame.
    #[inline]
    pub const fn is_trailers(&self) -> bool {
        matches!(self.repr, Repr::Trailers(_))
    }

    /// Returns reference of the data if this is a DATA frame.
    #[inline]
    pub const fn as_data(&self) -> Option<&T> {
        match &self.repr {
            Repr::Data(data) => Some(data),
            Repr::Trailers(_) => None,
        }
    }

    /// Returns reference to the trailers if this is a trailers frame.
    #[inline]
    pub const fn as_trailer(&self) -> Option<&HeaderMap> {
        match &self.repr {
            Repr::Trailers(trailer) => Some(trailer),
            Repr::Data(_) => None,
        }
    }

    /// Returns mutable reference of the data if this is a DATA frame.
    #[inline]
    pub const fn as_mut_data(&mut self) -> Option<&mut T> {
        match &mut self.repr {
            Repr::Data(data) => Some(data),
            Repr::Trailers(_) => None,
        }
    }

    /// Returns mutable reference to the trailers if this is a trailers frame.
    #[inline]
    pub const fn as_mut_trailer(&mut self) -> Option<&mut HeaderMap> {
        match &mut self.repr {
            Repr::Trailers(trailer) => Some(trailer),
            Repr::Data(_) => None,
        }
    }

    /// Consumes self into the bytes of the DATA frame.
    #[inline]
    pub fn into_data(self) -> Result<T, Self> {
        match self.repr {
            Repr::Data(data) => Ok(data),
            Repr::Trailers(_) => Err(self),
        }
    }

    /// Consumes self into the trailers of the trailers frame.
    #[inline]
    pub fn into_trailers(self) -> Result<HeaderMap, Self> {
        match self.repr {
            Repr::Trailers(trailers) => Ok(trailers),
            Repr::Data(_) => Err(self),
        }
    }
}
