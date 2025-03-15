use super::{Body, FromRequest, FromRequestParts, Request};
use crate::util::response::BadRequest;
use bytes::Bytes;
use http::request;
use std::{
    convert::Infallible,
    future::{ready, Ready},
};

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
            inner: Collect<Body>,
        }
    }

    impl BytesFuture {
        pub(super) fn new(inner: Body) -> BytesFuture {
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
            inner: Collect<Body>,
        }
    }

    impl StringFuture {
        pub(super) fn new(inner: Body) -> Self {
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
    pub enum StringFutureError {
        Hyper(hyper::Error),
        Utf8(FromUtf8Error),
    }

    impl std::fmt::Display for StringFutureError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            use std::fmt::Display;
            match self {
                Self::Hyper(hyper) => Display::fmt(hyper, f),
                Self::Utf8(utf8) => Display::fmt(utf8, f),
            }
        }
    }

    impl std::fmt::Debug for StringFutureError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            use std::fmt::Debug;
            match self {
                Self::Hyper(hyper) => Debug::fmt(hyper, f),
                Self::Utf8(utf8) => Debug::fmt(utf8, f),
            }
        }
    }
}


