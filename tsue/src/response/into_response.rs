use bytes::{Bytes, BytesMut};
use http::{header::CONTENT_TYPE, response, HeaderName, HeaderValue, StatusCode};
use mime::Mime;

use super::{IntoResponse, IntoResponseParts, Parts, Response};

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

// ===== Foreign Implementation =====

/// Anything that implement [`IntoResponseParts`] also implement [`IntoResponse`].
impl<R> IntoResponse for R
where
    R: IntoResponseParts,
{
    fn into_response(self) -> Response {
        let (mut parts, body) = Response::default().into_parts();
        parts = self.into_response_parts(parts);
        Response::from_parts(parts, body)
    }
}

impl IntoResponseParts for () {
    fn into_response_parts(self, parts: Parts) -> Parts {
        parts
    }
}

part!(std::convert::Infallible, (self) => match self { });
part!(StatusCode, mut (self,parts) => parts.status = self);

impl<T, E> IntoResponse for Result<T, E>
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

impl IntoResponse for serde_json::Error {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

impl IntoResponse for hyper::Error {
    fn into_response(self) -> Response {
        match self {
            me if me.is_parse() => StatusCode::BAD_REQUEST.into_response(),
            me if me.is_timeout() => StatusCode::REQUEST_TIMEOUT.into_response(),
            _err => {
                #[cfg(feature = "log")]
                log::error!("{_err}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            },
        }
    }
}

// ===== Headers =====

impl IntoResponseParts for (&'static str, &'static str) {
    fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
        parts.headers.append(
            HeaderName::from_static(self.0),
            HeaderValue::from_static(self.1),
        );
        parts
    }
}

impl<const N: usize> IntoResponseParts for [(&'static str, &'static str); N] {
    fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
        for (key, val) in self {
            parts
                .headers
                .append(HeaderName::from_static(key), HeaderValue::from_static(val));
        }
        parts
    }
}

impl<const N: usize> IntoResponseParts for [(&'static str, HeaderValue); N] {
    fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
        for (key, val) in self {
            parts
                .headers
                .append(HeaderName::from_static(key), val);
        }
        parts
    }
}

impl<const N: usize> IntoResponseParts for [(HeaderName, &'static str); N] {
    fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
        for (key, val) in self {
            parts.headers.append(key, HeaderValue::from_static(val.into()));
        }
        parts
    }
}

impl<const N: usize> IntoResponseParts for [(HeaderName, HeaderValue); N] {
    fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
        for (key, val) in self {
            parts.headers.append(key, val);
        }
        parts
    }
}

impl IntoResponseParts for Mime {
    fn into_response_parts(self, mut parts: Parts) -> Parts {
        parts
            .headers
            .insert(CONTENT_TYPE, self.as_ref().parse().unwrap());
        parts
    }
}

// ===== Body Implementations =====

res!(Bytes, self => Response::new(self.into()));
res!(Vec<u8>, self => Response::new(self.into()));
res!(BytesMut, self => Response::new(self.freeze().into()));
res!(Response, self => self);
res!(&'static str, self => IntoResponse::into_response((
    [(CONTENT_TYPE, "text/plain; charset=utf-8")],
    Bytes::from_static(self.as_bytes())
)));
res!(String, self => IntoResponse::into_response((
    [(CONTENT_TYPE, "text/plain; charset=utf-8")],
    Bytes::from(self)
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

into_response_tuple!(R1, R2, R3, R4, R5, R6, R7, R8,);
