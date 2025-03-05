//! [`FromRequest`] and [`FromRequestParts`] trait
use std::{convert::Infallible, future::{ready, Ready}};
use super::{into_response::IntoResponse, ReqBody, Request};
use crate::util::response::BadRequest;
use bytes::Bytes;
use http::request;

// NOTE: Previously, `FromRequest` only accept mutable reference of `request::Parts`
// that allow `IntoResponse` access it, things get absurdly complicated realy quick
// when we have to carry around `request::Parts`, and it makes `IntoResponse`
// not portable because it require `request::Part` to call it
// For now, use something like `Responder` to build response which come from function
// argument which have access to `request::Parts`

// NOTE:
// using Pin<Box> in associated type is worth it instead of impl Future,
// because it can be referenced externally
// [issue](#63063 <https://github.com/rust-lang/rust/issues/63063>)

/// Type that can be constructed from request
///
/// this trait is used as request handler parameters
pub trait FromRequest: Sized {
    type Error: IntoResponse;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request(req: Request) -> Self::Future;
}

/// Type that can be constructed from request parts
///
/// this trait is used as request handler parameters
pub trait FromRequestParts: Sized {
    type Error: IntoResponse;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request_parts(parts: &mut request::Parts) -> Self::Future;
}

/// anything that implement `FromRequestParts` also implement `FromRequest`
impl<F> FromRequest for F
where
    F: FromRequestParts
{
    type Error = <F as FromRequestParts>::Error;
    type Future = <F as FromRequestParts>::Future;

    fn from_request(req: Request) -> Self::Future {
        Self::from_request_parts(&mut req.into_parts().0)
    }
}

impl FromRequestParts for () {
    type Error = Infallible;
    type Future = Ready<Result<Self,Self::Error>>;

    fn from_request_parts(_: &mut request::Parts) -> Self::Future {
        ready(Ok(()))
    }
}

impl FromRequestParts for http::Method {
    type Error = Infallible;
    type Future = Ready<Result<Self,Self::Error>>;

    fn from_request_parts(parts: &mut request::Parts) -> Self::Future {
        ready(Ok(parts.method.clone()))
    }
}

impl FromRequestParts for http::Uri {
    type Error = Infallible;
    type Future = Ready<Result<Self,Self::Error>>;

    fn from_request_parts(parts: &mut request::Parts) -> Self::Future {
        ready(Ok(parts.uri.clone()))
    }
}

macro_rules! from_request {
    ($self:ty, $($id:ident = $t:ty;)* ($req:pat) => $body: expr) => {
        impl FromRequest for $self {
            $(type $id = $t;)*
            fn from_request($req: Request) -> Self::Future {
                $body
            }
        }
    };
}


#[doc(inline)]
pub use bytes_future::BytesFuture;
from_request! {
    Bytes,
    Error = BadRequest<hyper::Error>;
    Future = BytesFuture;
    (req) => BytesFuture::new(req.into_body())
}

#[doc(inline)]
pub use string_future::{StringFuture, StringFutureError};
from_request! {
    String,
    Error = BadRequest<StringFutureError>;
    Future = StringFuture;
    (req) => StringFuture::new(req.into_body())
}

mod bytes_future {
    use super::*;
    use http_body_util::{combinators::Collect, BodyExt};

    pin_project_lite::pin_project! {
        /// future returned from [`Bytes`] implementation of [`FromRequest`]
        pub struct BytesFuture {
            #[pin]
            inner: Collect<ReqBody>,
        }
    }

    impl BytesFuture {
        pub(super) fn new(inner: ReqBody) -> BytesFuture {
            Self { inner: inner.collect() }
        }
    }

    impl Future for BytesFuture {
        type Output = Result<Bytes, BadRequest<hyper::Error>>;

        fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            match self.project().inner.poll(cx) {
                Ready(Ok(ok)) => Ready(Ok(ok.to_bytes())),
                Ready(Err(err)) => Ready(Err(err.into())),
                Pending => Pending
            }
        }
    }
}

mod string_future {
    use super::*;
    use http_body_util::{combinators::Collect, BodyExt};
    use std::string::FromUtf8Error;

    pin_project_lite::pin_project! {
        /// future returned from [`String`] implementation of [`FromRequest`]
        pub struct StringFuture {
            #[pin]
            inner: Collect<ReqBody>,
        }
    }

    impl StringFuture {
        pub(super) fn new(inner: ReqBody) -> Self {
            Self { inner: inner.collect() }
        }
    }

    impl Future for StringFuture {
        type Output = Result<String, BadRequest<StringFutureError>>;

        fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            match self.project().inner.poll(cx) {
                Ready(Ok(ok)) => match String::from_utf8(Vec::from(ok.to_bytes())) {
                    Ok(ok) => Ready(Ok(ok)),
                    Err(err) => Ready(Err(StringFutureError::Utf8(err).into())),
                },
                Ready(Err(err)) => Ready(Err(StringFutureError::Hyper(err).into())),
                Pending => Pending
            }
        }
    }

    /// error returned from [`String`] implementation of [`FromRequest`]
    #[derive(thiserror::Error, Debug)]
    pub enum StringFutureError {
        #[error(transparent)]
        Hyper(hyper::Error),
        #[error(transparent)]
        Utf8(FromUtf8Error),
    }
}

