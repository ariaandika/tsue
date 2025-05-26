use bytes::{Bytes, BytesMut};
use http::{HeaderName, HeaderValue, StatusCode, header::CONTENT_TYPE, response};

use super::{IntoResponse, IntoResponseParts, Parts, Response};

use macros::{headers, into_response_tuple, part, res};

// ===== Blanket Implementation =====

/// Anything that implement [`IntoResponseParts`] also implement [`IntoResponse`].
impl<R: IntoResponseParts> IntoResponse for R {
    fn into_response(self) -> Response {
        let (mut parts, body) = Response::default().into_parts();
        parts = self.into_response_parts(parts);
        Response::from_parts(parts, body)
    }
}

// ===== Foreign Implementation =====

part!((), (self,parts) => {});
part!(std::convert::Infallible, (self) => match self { });
part!(StatusCode, mut (self,parts) => parts.status = self);

into_response_tuple!(R1, R2, R3, R4, R5, R6, R7, R8,);

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

impl IntoResponse for hyper::Error {
    fn into_response(self) -> Response {
        match self {
            me if me.is_parse() => StatusCode::BAD_REQUEST.into_response(),
            me if me.is_timeout() => StatusCode::REQUEST_TIMEOUT.into_response(),
            _err => {
                #[cfg(feature = "log")]
                log::error!("{_err}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
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

headers! {
    |key: &'static str|HeaderName::from_static(key);
    |val: &'static str|HeaderValue::from_static(val);
}

headers! {
    |key: &'static str|HeaderName::from_static(key);
    |val: HeaderValue|val;
}

headers! {
    |key: HeaderName|key;
    |val: &'static str|HeaderValue::from_static(val);
}

headers! {
    |key: HeaderName|key;
    |val: HeaderValue|val;
}

// ===== Body Implementations =====

const UTF8: [(HeaderName,HeaderValue);1] = [(CONTENT_TYPE,HeaderValue::from_static("application/x-www-form-urlencoded"))];

res!(&'static [u8], self => Response::new(Bytes::from_static(self).into()));
res!(Bytes, self => Response::new(self.into()));
res!(Vec<u8>, self => Response::new(self.into()));
res!(BytesMut, self => Response::new(self.freeze().into()));
res!(Response, self => self);
res!(&'static str, self => (UTF8,Bytes::from_static(self.as_bytes())).into_response());
res!(String, self => (UTF8,Bytes::from(self)).into_response());

// ===== Macros =====

mod macros {
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

    macro_rules! headers {
        (
            |$h1:ident: $t1:ty|$b1:expr;
            |$h2:ident: $t2:ty|$b2:expr;
        ) => {
            impl<const N: usize> IntoResponseParts for [($t1, $t2); N] {
                fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
                    for ($h1, $h2) in self {
                        parts.headers.append($b1, $b2);
                    }
                    parts
                }
            }
        };
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

    pub(crate) use {part, res, into_response_tuple, headers};
}

