use super::{IntoResponse, IntoResponseParts, Parts, Response};
use bytes::{Bytes, BytesMut};
use http::{response, HeaderMap, HeaderValue, StatusCode};

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
part!(HeaderMap, mut,mut (self,parts) => {
    const PLACEHOLDER: HeaderValue = HeaderValue::from_static("deez");
    for (key,val) in (&mut self).into_iter() {
        parts.headers.insert(key, std::mem::replace(val, PLACEHOLDER));
    }
});

res!(Bytes, self => Response::new(self.into()));
res!(BytesMut, self => Response::new(self.freeze().into()));
res!(Response, self => self);
res!(&'static str, self => IntoResponse::into_response((
    [("Content-Type","text/plain")], Bytes::from_static(self.as_bytes())
)));
res!(String, self => IntoResponse::into_response((
    [("Content-Type","text/plain")], Bytes::from(self)
)));

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

