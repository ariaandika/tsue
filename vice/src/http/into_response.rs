//! the [`IntoResponse`] and [`IntoResponseParts`] trait
use super::Response;
use bytes::Bytes;
use http::{request, response, HeaderMap, HeaderValue, StatusCode};

/// a type that can be converted into response
///
/// this trait is used as request handler return type
pub trait IntoResponse {
    fn into_response(self, req_parts: &mut request::Parts) -> Response;
}

/// a type that can be converted into response parts
///
/// this trait is used as request handler return type
pub trait IntoResponseParts {
    fn into_response_parts(self, parts: response::Parts) -> response::Parts;
}

impl<R> IntoResponse for R
where
    R: IntoResponseParts,
{
    fn into_response(self, _: &mut request::Parts) -> Response {
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
    fn into_response(self, req_parts: &mut request::Parts) -> Response {
        match self {
            Ok(ok) => ok.into_response(req_parts),
            Err(err) => err.into_response(req_parts),
        }
    }
}

// NOTE: foreign implementation

macro_rules! res {
    ($target:ty, $self:ident, $req:pat => $body:expr) => {
        impl IntoResponse for $target {
            fn into_response($self, $req: &mut request::Parts) -> Response {
                $body
            }
        }
    };
}

res!((), self, _ => <_>::default());
res!(std::convert::Infallible, self, _ => match self { });
res!(Bytes, self, _ => Response::new(self.into()));
res!(Response, self, _ => self);
res!(&'static str, self, parts => IntoResponse::into_response((
    ("Content-Type","text/plain"), Bytes::from_static(self.as_bytes())
),parts));
res!(String, self, parts => IntoResponse::into_response((
    ("Content-Type","text/plain"), Bytes::from(self)
),parts));

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
            fn into_response(self, req_parts: &mut request::Parts) -> Response {
                #![allow(non_snake_case)]
                let ($($r,)*r) = self;
                let (mut parts,body) = r.into_response(req_parts).into_parts();
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

