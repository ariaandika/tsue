use super::{FromRequest, FromRequestParts, Parts, Request};
use crate::{http::Method, response::BadRequest};
use std::{
    convert::Infallible,
    future::{ready, Ready},
    io,
};

// NOTE:
// using Pin<Box> in association type is worth it instead of impl Future,
// because it can be referenced externally

macro_rules! from_parts {
    ($self:ty, $($id:ident = $t:ty;)* ($parts:pat) => $body: expr) => {
        impl FromRequestParts for $self {
            $(type $id = $t;)*

            fn from_request_parts($parts: &mut Parts) -> Self::Future {
                $body
            }
        }
    };
}

from_parts! {
    Method,
    Error = Infallible;
    Future = Ready<Result<Self,Infallible>>;
    (parts) => ready(Ok(parts.method))
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

from_request! {
    (),
    Error = Infallible;
    Future = Ready<Result<Self,Infallible>>;
    (_) => ready(Ok(()))
}

from_request! {
    Request,
    Error = Infallible;
    Future = Ready<Result<Self,Infallible>>;
    (req) => ready(Ok(req))
}

// from_request! {
//     BytesMut,
//     Error = Infallible;
//     Future = MapInfallible<StreamFuture<BytesMut>>;
//     (req) => req.into_body().bytes_mut().map_infallible()
// }

// from_request! {
//     Bytes,
//     Error = io::Error;
//     Future = futures::MapOk<<BytesMut as FromRequest>::Future, fn(BytesMut) -> Bytes>;
//     (req) => BytesMut::from_request(req).map_ok(BytesMut::freeze as _)
// }
//
// from_request! {
//     Vec<u8>,
//     Error = io::Error;
//     Future = futures::MapOk<<BytesMut as FromRequest>::Future, fn(BytesMut) -> Vec<u8>>;
//     (req) => BytesMut::from_request(req).map_ok(Into::into as _)
// }

from_request! {
    String,
    Error = BadRequest<io::Error>;
    Future = Ready<Result<Self,BadRequest<io::Error>>>;
    (_req) => todo!()// BytesMut::from_request(req).map(|e|String::from_utf8(e?.into()).map_err(Into::into))
}

/*
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
*/
