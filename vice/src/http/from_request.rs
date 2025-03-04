//! the [`FromRequest`] and [`FromRequestParts`] trait
use super::{into_response::IntoResponse, ReqBody};
use crate::util::response::BadRequest;
use bytes::Bytes;
use http::request;
use http_body_util::BodyExt;

/// a type that can be constructed from request parts and body
///
/// this trait is used as request handler parameters
///
// previously, `FromRequest` accept the whole `Request` struct,
// now it only the parts, to allow request parts be accessed after handler
pub trait FromRequest: Sized {
    type Error: IntoResponse;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request(parts: &mut request::Parts, body: ReqBody) -> Self::Future;
}

/// a type that can be constructed from request parts
///
/// this trait is used as request handler parameters
pub trait FromRequestParts: Sized {
    type Error: IntoResponse;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request_parts(parts: &mut request::Parts) -> Self::Future;
}

// NOTE:
// using Pin<Box> in association type is worth it instead of impl Future,
// because it can be referenced externally

/// anything that implement `FromRequestParts` also implement `FromRequest`
impl<F> FromRequest for F
where
    F: FromRequestParts
{
    type Error = <F as FromRequestParts>::Error;
    type Future = <F as FromRequestParts>::Future;

    fn from_request(parts: &mut request::Parts, _: ReqBody) -> Self::Future {
        Self::from_request_parts(parts)
    }
}

impl FromRequestParts for () {
    type Error = std::convert::Infallible;
    type Future = std::future::Ready<Result<Self,Self::Error>>;

    fn from_request_parts(_: &mut request::Parts) -> Self::Future {
        std::future::ready(Ok(()))
    }
}

impl FromRequestParts for http::Method {
    type Error = std::convert::Infallible;
    type Future = std::future::Ready<Result<Self,Self::Error>>;

    fn from_request_parts(parts: &mut request::Parts) -> Self::Future {
        std::future::ready(Ok(parts.method.clone()))
    }
}

macro_rules! from_request {
    ($self:ty, $($id:ident = $t:ty;)* ($parts:ident) => $body: expr) => {
        from_request!($self, $($id = $t;)* ($parts, _) => $body);
    };
    ($self:ty, $($id:ident = $t:ty;)* ($parts:pat, $arg2:pat) => $body: expr) => {
        impl FromRequest for $self {
            $(type $id = $t;)*
            fn from_request($parts: &mut request::Parts, $arg2: ReqBody) -> Self::Future {
                $body
            }
        }
    };
}


#[doc(inline)]
pub use body_future::BodyFuture;
from_request! {
    Bytes,
    Error = BadRequest<hyper::Error>;
    Future = BodyFuture;
    (_, body) => BodyFuture::new(body)
}

#[doc(inline)]
pub use body_string_future::{BodyStringFuture, BodyStringError};
from_request! {
    String,
    Error = BadRequest<BodyStringError>;
    Future = BodyStringFuture;
    (_, body) => BodyStringFuture::new(body)
}

mod body_future {
    use super::*;
    use http_body_util::combinators::Collect;

    pin_project_lite::pin_project! {
        /// future returned from [`FromRequest`] implementation of [`Bytes`]
        ///
        /// [`Bytes`]: super::Bytes
        /// [`FromRequest`]: super::FromRequest
        pub struct BodyFuture {
            #[pin]
            inner: Collect<ReqBody>,
        }
    }

    impl BodyFuture {
        pub(crate) fn new(inner: ReqBody) -> BodyFuture {
            Self { inner: inner.collect() }
        }
    }

    impl Future for BodyFuture {
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

mod body_string_future {
    use super::*;
    use http_body_util::combinators::Collect;
    use std::string::FromUtf8Error;

    pin_project_lite::pin_project! {
        /// future returned from [`FromRequest`] implementation of [`String`]
        ///
        /// [`FromRequest`]: super::FromRequest
        pub struct BodyStringFuture {
            #[pin]
            inner: Collect<ReqBody>,
        }
    }

    impl BodyStringFuture {
        pub(crate) fn new(inner: ReqBody) -> Self {
            Self { inner: inner.collect() }
        }
    }

    impl Future for BodyStringFuture {
        type Output = Result<String, BadRequest<BodyStringError>>;

        fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            match self.project().inner.poll(cx) {
                Ready(Ok(ok)) => match String::from_utf8(Vec::from(ok.to_bytes())) {
                    Ok(ok) => Ready(Ok(ok)),
                    Err(err) => Ready(Err(BodyStringError::Utf8(err).into())),
                },
                Ready(Err(err)) => Ready(Err(BodyStringError::Hyper(err).into())),
                Pending => Pending
            }
        }
    }

    /// error returned from [`FromRequest`] implementation of [`String`]
    ///
    /// [`FromRequest`]: super::FromRequest
    #[derive(thiserror::Error, Debug)]
    pub enum BodyStringError {
        #[error(transparent)]
        Hyper(hyper::Error),
        #[error(transparent)]
        Utf8(FromUtf8Error),
    }
}

