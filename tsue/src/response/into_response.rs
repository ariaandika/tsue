use bytes::{Bytes, BytesMut};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, response};
use mime::Mime;
use serde::Serialize;

use super::{Html, IntoResponse, IntoResponseParts, Parts, Redirect, Response};
use crate::extractor::Json;

macro_rules! part {
    ($target:ty, $($mut:ident)* $(, $mut2:ident)* ($self:ident) => $body:expr) => {
        part!(@ $target, $($mut)* $(, $mut2)* ($self,_part) => $body);
    };
    ($target:ty, $($mut:ident)* $(, $mut2:ident)* ($self:ident,$part:ident) => $body:expr) => {
        part!(@ $target, $($mut)* $(, $mut2)* ($self,$part) => { $body; $part });
    };
    (@ $target:ty, $($mut:ident)* $(, $mut2:ident)* ($self:ident,$part:ident) => $body:expr) => {
        impl IntoResponseParts for $target {
            fn into_response_parts($($mut2)* $self, $($mut)* $part: Parts) -> Parts {
                $body
            }
        }
    };
}

macro_rules! res {
    ($target:ty, $self:ident => $body:expr) => {
        impl IntoResponse for $target {
            fn into_response($self) -> Response {
                $body
            }
        }
    };
}

/// Anything that implement [`IntoResponseParts`] also implement [`IntoResponse`].
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

impl IntoResponseParts for (&'static str, &'static str) {
    fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
        parts.headers.insert(
            HeaderName::from_static(self.0),
            HeaderValue::from_static(self.1)
        );
        parts
    }
}

impl<const N: usize, T> IntoResponseParts for [(&'static str, T);N]
where
    T: Into<Bytes>,
{
    fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
        for (key,val) in self {
            if let Ok(value) = HeaderValue::from_maybe_shared(val.into()) {
                parts.headers.insert(key, value);
            }
        }
        parts
    }
}

part!((), (self,parts) => ());
part!(std::convert::Infallible, (self) => match self { });
part!(StatusCode, mut (self,parts) => parts.status = self);
part!(Mime, mut (self,parts) => parts.headers.insert(
    HeaderName::from_static("content-type"), HeaderValue::from_str(self.as_ref()).unwrap()
));
part!(HeaderMap, mut,mut (self,parts) => {
    const PLACEHOLDER: HeaderValue = HeaderValue::from_static("deez");
    for (key,val) in (&mut self).into_iter() {
        parts.headers.insert(key, std::mem::replace(val, PLACEHOLDER));
    }
});

res!(Bytes, self => Response::new(self.into()));
res!(Vec<u8>, self => Response::new(self.into()));
res!(BytesMut, self => Response::new(self.freeze().into()));
res!(Response, self => self);
res!(&'static str, self => IntoResponse::into_response((
    ("content-type","text/plain; charset=utf-8"), Bytes::from_static(self.as_bytes())
)));
res!(String, self => IntoResponse::into_response((
    ("content-type","text/plain; charset=utf-8"), Bytes::from(self)
)));
res!(Redirect, self => IntoResponse::into_response((
    [("location",self.location)], self.status,
)));

impl<T> IntoResponse for Html<T>
where
    T: Into<Bytes>,
{
    fn into_response(self) -> Response {
        (("content-type", "text/html; charset=utf-8"), self.0.into()).into_response()
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(ok) => (("content-type", "application/json"), ok).into_response(),
            Err(_err) => {
                #[cfg(feature = "log")]
                log::error!("failed to serialize json response: {_err}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
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

