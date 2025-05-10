//! `Futures` and `Error` types.
use bytes::{BufMut, Bytes, BytesMut};
use http::{Extensions, HeaderMap, Method, StatusCode, Uri, Version};
use http_body::Body as _;
use serde::de::DeserializeOwned;
use std::{
    convert::Infallible,
    fmt,
    future::{Ready, ready},
    marker::PhantomData,
    mem,
    pin::Pin,
    string::FromUtf8Error,
    task::{Context, Poll, ready},
};

use super::{Body, FromRequest, FromRequestParts, Parts, Request};
use crate::{
    extractor::Json,
    response::{IntoResponse, Response},
};

macro_rules! from_parts {
    ($self:ty, ($parts:pat) => $body: expr) => {
        from_parts!(
            $self,Error=Infallible;Future=Ready<Result<Self,Self::Error>>;
            ($parts) => ready(Ok($body))
        );
    };
    ($self:ty, $($id:ident = $t:ty;)* ($parts:pat) => $body: expr) => {
        impl FromRequestParts for $self {
            $(type $id = $t;)*
            fn from_request_parts($parts: &mut Parts) -> Self::Future { $body }
        }
    };
}

macro_rules! from_req {
    ($self:ty, ($req:pat) => $body:expr) => {
        from_req!(
            $self,Error=Infallible;Future=Ready<Result<Self,Self::Error>>;
            ($req) => ready(Ok($body))
        );
    };
    ($self:ty, $($id:ident = $t:ty;)* ($req:pat) => $body: expr) => {
        impl FromRequest for $self {
            $(type $id = $t;)*
            fn from_request($req: Request) -> Self::Future { $body }
        }
    };
}

/// Anything that implement `FromRequestParts` also implement `FromRequest`.
impl<F> FromRequest for F where F: FromRequestParts {
    type Error = <F as FromRequestParts>::Error;

    type Future = <F as FromRequestParts>::Future;

    fn from_request(req: Request) -> Self::Future {
        Self::from_request_parts(&mut req.into_parts().0)
    }
}

from_parts!((), (_) => ());
from_parts!(Method, (parts) => parts.method.clone());
from_parts!(Uri, (parts) => parts.uri.clone());
from_parts!(Version, (parts) => parts.version);
from_parts!(HeaderMap, (parts) => parts.headers.clone());
from_parts!(Extensions, (parts) => parts.extensions.clone());
from_parts!(Parts, (parts) => parts.clone());

from_req!(Request, (req) => req);
from_req!(Body, (req) => req.into_body());

// ===== Body Implementations =====

from_req! {
    Bytes,
    Error = BytesFutureError;
    Future = BytesFuture;
    (req) => BytesFuture {
        buffer: BytesMut::new(),
        inner: req.into_body(),
    }
}

from_req! {
    String,
    Error = StringFutureError;
    Future = StringFuture;
    (req) => StringFuture {
        f: BytesFuture {
            buffer: BytesMut::new(),
            inner: req.into_body(),
        }
    }
}

impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned,
{
    type Error = JsonFutureError;
    type Future = JsonFuture<T>;

    fn from_request(req: Request) -> Self::Future {
        JsonFuture {
            f: BytesFuture {
                buffer: BytesMut::new(),
                inner: req.into_body(),
            },
            _p: PhantomData,
        }
    }
}

// ===== Future Implementations =====

/// Future returned from [`Bytes`] implementation of [`FromRequest`].
pub struct BytesFuture {
    buffer: BytesMut,
    inner: Body,
}

/// Future returned from [`String`] implementation of [`FromRequest`].
pub struct StringFuture {
    f: BytesFuture,
}

/// Future returned from [`Json`] implementation of [`FromRequest`].
pub struct JsonFuture<T> {
    f: BytesFuture,
    _p: PhantomData<T>,
}

impl<T> Unpin for JsonFuture<T> { }

impl Future for BytesFuture {
    type Output = Result<Bytes, BytesFutureError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();
        let mut f = Pin::new(&mut me.inner);

        while let Some(frame) = ready!(f.as_mut().poll_frame(cx)?) {
            if let Ok(data) = frame.into_data() {
                me.buffer.put(data);
            }
        }

        Poll::Ready(Ok(mem::take(&mut me.buffer).freeze()))
    }
}

impl Future for StringFuture {
    type Output = Result<String, StringFutureError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ok = ready!(Pin::new(&mut self.f).poll(cx)?);
        Poll::Ready(Ok(String::from_utf8(ok.into())?))
    }
}

impl<T> Future for JsonFuture<T>
where
    T: DeserializeOwned,
{
    type Output = Result<Json<T>, JsonFutureError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ok = ready!(Pin::new(&mut self.f).poll(cx)?);
        Poll::Ready(Ok(Json(serde_json::from_slice::<T>(&ok)?)))
    }
}

// ===== Errors =====

macro_rules! from {
    ($id:ident, $fr:ty: $pat:pat => $body:expr) => {
        impl From<$fr> for $id {
            fn from($pat: $fr) -> Self {
                $body
            }
        }
    };
}

/// Error that can be returned by [`BytesFuture`].
#[derive(Debug)]
pub struct BytesFutureError(hyper::Error);

from!(BytesFutureError, hyper::Error: e => Self(e));

impl IntoResponse for BytesFutureError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.0.to_string()).into_response()
    }
}

impl std::error::Error for BytesFutureError { }

impl fmt::Display for BytesFutureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Error that can be returned by [`StringFuture`].
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

impl std::error::Error for StringFutureError { }

impl fmt::Display for StringFutureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hyper(hyper) => hyper.fmt(f),
            Self::Utf8(utf8) => utf8.fmt(f),
        }
    }
}

/// Error that can be returned by [`JsonFuture`].
#[derive(Debug)]
pub enum JsonFutureError {
    Hyper(hyper::Error),
    Serde(serde_json::Error),
}

from!(JsonFutureError, BytesFutureError: e => Self::Hyper(e.0));
from!(JsonFutureError, serde_json::Error: e => Self::Serde(e));

impl IntoResponse for JsonFutureError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

impl std::error::Error for JsonFutureError { }

impl fmt::Display for JsonFutureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hyper(hyper) => hyper.fmt(f),
            Self::Serde(serde) => serde.fmt(f),
        }
    }
}

