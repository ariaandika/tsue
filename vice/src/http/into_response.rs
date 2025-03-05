//! the [`IntoResponse`] and [`IntoResponseParts`] trait
use super::Response;
use bytes::Bytes;
use http::{response, HeaderMap, HeaderValue, StatusCode};

/// a type that can be converted into response
///
/// this trait is used as request handler return type
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

/// a type that can be converted into response parts
///
/// this trait is used as request handler return type
pub trait IntoResponseParts {
    fn into_response_parts(self, parts: response::Parts) -> response::Parts;
}

/// anything that implement `IntoResponseParts` also implement `IntoResponse`
impl<R> IntoResponse for R
where
    R: IntoResponseParts,
{
    fn into_response(self) -> Response {
        let (mut parts,body) = Response::default().into_parts();
        parts = self.into_response_parts(parts);
        Response::from_parts(parts, body)
    }
}

impl<T,E> IntoResponse for Result<T,E>
where
    T: IntoResponse,
    E: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Ok(ok) => ok.into_response(),
            Err(err) => err.into_response(),
        }
    }
}

// NOTE: foreign implementation

macro_rules! res {
    ($target:ty, $self:ident => $body:expr) => {
        impl IntoResponse for $target {
            fn into_response($self) -> Response {
                $body
            }
        }
    };
}

res!((), self => <_>::default());
res!(std::convert::Infallible, self => match self { });
res!(Bytes, self => Response::new(self.into()));
res!(Response, self => self);
res!(&'static str, self => IntoResponse::into_response((
    ("Content-Type","text/plain"), Bytes::from_static(self.as_bytes())
)));
res!(String, self => IntoResponse::into_response((
    ("Content-Type","text/plain"), Bytes::from(self)
)));

impl IntoResponseParts for StatusCode {
    fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
        parts.status = self;
        parts
    }
}

impl IntoResponseParts for HeaderMap {
    fn into_response_parts(mut self, mut parts: response::Parts) -> response::Parts {
        const PLACEHOLDER: HeaderValue = HeaderValue::from_static("deez");
        for (key,val) in (&mut self).into_iter() {
            parts.headers.insert(key, std::mem::replace(val, PLACEHOLDER));
        }
        parts
    }
}

impl<T> IntoResponseParts for (&'static str, T)
where
    T: Into<Bytes>,
{
    fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
        if let Ok(value) = HeaderValue::from_maybe_shared(self.1.into()) {
            parts.headers.insert(self.0, value);
        }
        parts
    }
}

macro_rules! into_response_tuple {
    (@$($r:ident,)*) => {
        impl<$($r,)*R> IntoResponse for ($($r,)*R)
        where
            $($r: IntoResponseParts,)*
            R: IntoResponse,
        {
            fn into_response(self) -> Response {
                #![allow(non_snake_case)]
                let ($($r,)*r) = self;
                let (mut parts,body) = r.into_response().into_parts();
                $(parts = $r.into_response_parts(parts);)*
                Response::from_parts(parts, body)
            }
        }
    };
    () => { };
    ($r:ident,$($r2:ident,)*) => {
        into_response_tuple!($($r2,)*);
        into_response_tuple!(@ $r, $($r2,)*);
    };
}

into_response_tuple!(R1,R2,R3,R4,R5,R6,R7,R8,);

