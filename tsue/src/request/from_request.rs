//! `Futures` and `Error` types.
use bytes::{Bytes, BytesMut};
use futures_util::{FutureExt, future::Map};
use http::{Extensions, HeaderMap, Method, StatusCode, Uri, Version};
use http_body_util::{BodyExt, Collected, combinators::Collect};
use std::{
    convert::Infallible,
    fmt,
    future::{Ready, ready},
    string::FromUtf8Error,
};

use super::{Body, FromRequest, FromRequestParts, Parts, Request};
use crate::response::{IntoResponse, Response};

// ===== Macros =====

macro_rules! from_parts {
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

macro_rules! from_req {
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

from_parts!((), |_| ());
from_parts!(Method, |parts| parts.method.clone());
from_parts!(Uri, |parts| parts.uri.clone());
from_parts!(Version, |parts| parts.version);
from_parts!(HeaderMap, |parts| parts.headers.clone());
from_parts!(Extensions, |parts| parts.extensions.clone());
from_parts!(Parts, |parts| parts.clone());

from_req!(Request, |req| req);
from_req!(Body, |req| req.into_body());

// ===== Body Implementations =====

type BytesFuture = Map<
    Collect<Body>,
    fn(Result<Collected<Bytes>, hyper::Error>) -> Result<Bytes, BytesFutureError>,
>;

from_req! {
    Bytes,
    Error = BytesFutureError;
    Future = BytesFuture;
    |req|req.into_body().collect().map(|e|Ok(e?.to_bytes()))
}

type BytesMutFuture = Map<
    Collect<Body>,
    fn(Result<Collected<Bytes>, hyper::Error>) -> Result<BytesMut, BytesFutureError>,
>;

from_req! {
    BytesMut,
    Error = BytesFutureError;
    Future = BytesMutFuture;
    |req|req.into_body().collect().map(|e|Ok(e?.to_bytes().into()))
}

type StringFuture =
    Map<BytesFuture, fn(Result<Bytes, BytesFutureError>) -> Result<String, StringFutureError>>;

from_req! {
    String,
    Error = StringFutureError;
    Future = StringFuture;
    |req|Bytes::from_request(req).map(|e|Ok(String::from_utf8(e?.into())?))
}

// ===== Errors =====

/// Error that can be returned from [`Bytes`] implementation of [`FromRequest`].
#[derive(Debug)]
pub struct BytesFutureError(hyper::Error);

from!(BytesFutureError, hyper::Error: e => Self(e));

impl From<BytesFutureError> for hyper::Error {
    fn from(value: BytesFutureError) -> Self {
        value.0
    }
}

impl IntoResponse for BytesFutureError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.0.to_string()).into_response()
    }
}

impl std::error::Error for BytesFutureError {}

impl fmt::Display for BytesFutureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Error that can be returned from [`String`] implementation of [`FromRequest`].
#[derive(Debug)]
pub enum StringFutureError {
    Hyper(hyper::Error),
    Utf8(FromUtf8Error),
}

from!(StringFutureError, BytesFutureError: e => Self::Hyper(e.0));
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
            Self::Hyper(hyper) => hyper.fmt(f),
            Self::Utf8(utf8) => utf8.fmt(f),
        }
    }
}
