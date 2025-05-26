//! `Futures` and `Error` types.
use bytes::{Bytes, BytesMut};
use futures_util::{FutureExt, future::Map};
use http::{Extensions, HeaderMap, Method, StatusCode, Uri, Version};
use std::{
    convert::Infallible,
    fmt,
    future::{Ready, ready},
    string::FromUtf8Error,
};

use super::{FromRequest, FromRequestParts, Parts, Request};
use crate::{
    body::{Body, BodyError, Collect, Collected},
    response::{IntoResponse, Response},
};

use macros::{from, parts, req};

// ===== Blanket Implementation =====

/// Anything that implement [`FromRequestParts`] also implement [`FromRequest`].
impl<F> FromRequest for F
where
    F: FromRequestParts,
{
    type Error = <F as FromRequestParts>::Error;

    type Future = <F as FromRequestParts>::Future;

    fn from_request(req: Request) -> Self::Future {
        Self::from_request_parts(&mut req.into_parts().0)
    }
}

// ===== Foreign Implementation =====

parts!((), |_| ());
parts!(Method, |parts| parts.method.clone());
parts!(Uri, |parts| parts.uri.clone());
parts!(Version, |parts| parts.version);
parts!(HeaderMap, |parts| parts.headers.clone());
parts!(Extensions, |parts| parts.extensions.clone());
parts!(Parts, |parts| parts.clone());

req!(Request, |req| req);
req!(Body, |req| req.into_body());

// ===== Body Implementations =====

type BodyMap<O, E = BodyError> =
    Map<Collect, fn(Result<Collected, BodyError>) -> Result<O, E>>;

req! {
    Bytes,
    Error = BodyError;
    Future = BodyMap<Bytes>;
    |req|req.into_body().collect_body().map(|e|Ok(e?.into_bytes()))
}

req! {
    BytesMut,
    Error = BodyError;
    Future = BodyMap<BytesMut>;
    |req|req.into_body().collect_body().map(|e|Ok(e?.into_bytes_mut()))
}

req! {
    Box<[u8]>,
    Error = BodyError;
    Future = BodyMap<Box<[u8]>>;
    |req|req.into_body().collect_body().map(|e|Ok(Vec::from(e?.into_bytes_mut()).into_boxed_slice()))
}

req! {
    Vec<u8>,
    Error = BodyError;
    Future = BodyMap<Vec<u8>>;
    |req|req.into_body().collect_body().map(|e|Ok(e?.into_bytes_mut().into()))
}

req! {
    Box<str>,
    Error = StringFutureError;
    Future = BodyMap<Box<str>, StringFutureError>;
    |req|req.into_body().collect_body().map(|e|Ok(String::from_utf8(e?.into_bytes_mut().into())?.into_boxed_str()))
}

req! {
    String,
    Error = StringFutureError;
    Future = BodyMap<String, StringFutureError>;
    |req|req.into_body().collect_body().map(|e|Ok(String::from_utf8(e?.into_bytes_mut().into())?))
}

// ===== Errors =====

/// Error that can be returned from [`String`] implementation of [`FromRequest`].
#[derive(Debug)]
pub enum StringFutureError {
    Body(BodyError),
    Utf8(FromUtf8Error),
}

from!(StringFutureError, BodyError: e => Self::Body(e));
from!(StringFutureError, FromUtf8Error: e => Self::Utf8(e));

impl IntoResponse for StringFutureError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

impl std::error::Error for StringFutureError {}

impl fmt::Display for StringFutureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Body(body) => body.fmt(f),
            Self::Utf8(utf8) => utf8.fmt(f),
        }
    }
}

// ===== Macros =====

mod macros {
    macro_rules! parts {
        ($self:ty, |$parts:pat_param|$body:expr) => {
            impl FromRequestParts for $self {
                type Error = Infallible;
                type Future = Ready<Result<Self,Self::Error>>;
                fn from_request_parts($parts: &mut Parts) -> Self::Future { ready(Ok($body)) }
            }
        };
        ($self:ty, $($id:ident = $t:ty;)* |$parts:pat_param|$body:expr) => {
            impl FromRequestParts for $self {
                $(type $id = $t;)*
                fn from_request_parts($parts: &mut Parts) -> Self::Future { $body }
            }
        };
    }

    macro_rules! req {
        ($self:ty, |$req:pat_param|$body:expr) => {
            impl FromRequest for $self {
                type Error = Infallible;
                type Future = Ready<Result<Self,Self::Error>>;
                fn from_request($req: Request) -> Self::Future { ready(Ok($body)) }
            }
        };
        ($self:ty, $($id:ident = $t:ty;)* |$req:pat_param|$body:expr) => {
            impl FromRequest for $self {
                $(type $id = $t;)*
                fn from_request($req: Request) -> Self::Future { $body }
            }
        };
    }

    macro_rules! from {
        ($id:ident, $fr:ty: $pat:pat => $body:expr) => {
            impl From<$fr> for $id {
                fn from($pat: $fr) -> Self {
                    $body
                }
            }
        };
    }

    pub(crate) use {parts, req, from};
}

