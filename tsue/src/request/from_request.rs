use super::{Body, FromRequest, FromRequestParts, Parts, Request};
use bytes::Bytes;
use http::{Extensions, HeaderMap, Method, Uri, Version};
use std::{
    convert::Infallible,
    future::{ready, Ready},
    mem,
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

/// anything that implement `FromRequestParts` also implement `FromRequest`
impl<F> FromRequest for F where F: FromRequestParts {
    type Error = <F as FromRequestParts>::Error;
    type Future = <F as FromRequestParts>::Future;

    fn from_request(req: Request) -> Self::Future {
        Self::from_request_parts(&mut req.into_parts().0)
    }
}

from_parts!((), (_) => ());
from_parts!(Method, (parts) => parts.method.clone());
from_parts!(Uri, (parts) => mem::take(&mut parts.uri));
from_parts!(Version, (parts) => parts.version);
from_parts!(HeaderMap, (parts) => mem::take(&mut parts.headers));
from_parts!(Extensions, (parts) => mem::take(&mut parts.extensions));
from_parts!(Parts, (parts) => mem::replace(parts, Request::new(()).into_parts().0));

from_req!(Request, (req) => req);
from_req!(Body, (req) => req.into_body());

// Body Implementations

from_req! {
    Bytes,
    Error = BytesFutureError;
    Future = BytesFuture;
    (req) => BytesFuture::new(req.into_body())
}

from_req! {
    String,
    Error = StringFutureError;
    Future = StringFuture;
    (req) => StringFuture::new(req.into_body())
}

pub use bytes_future::{BytesFuture, BytesFutureError};
pub use string_future::{StringFuture, StringFutureError};

mod bytes_future {
    use super::*;
    use crate::response::{IntoResponse, Response};
    use http::StatusCode;
    use http_body_util::{combinators::Collect, BodyExt};

    pin_project_lite::pin_project! {
        /// Future returned from [`Bytes`] implementation of [`FromRequest`]
        pub struct BytesFuture {
            #[pin]
            inner: Collect<Body>,
        }
    }

    impl BytesFuture {
        pub fn new(inner: Body) -> BytesFuture {
            Self { inner: inner.collect() }
        }
    }

    impl Future for BytesFuture {
        type Output = Result<Bytes, BytesFutureError>;

        fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            match self.project().inner.poll(cx) {
                Ready(Ok(ok)) => Ready(Ok(ok.to_bytes())),
                Ready(Err(err)) => Ready(Err(BytesFutureError(err))),
                Pending => Pending
            }
        }
    }

    /// Error that can be returned by [`BytesFuture`]
    #[derive(Debug)]
    pub struct BytesFutureError(hyper::Error);

    impl IntoResponse for BytesFutureError {
        fn into_response(self) -> Response {
            (StatusCode::BAD_REQUEST,self.0.to_string()).into_response()
        }
    }

    impl std::error::Error for BytesFutureError { }
    impl std::fmt::Display for BytesFutureError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }
}

mod string_future {
    use super::*;
    use crate::response::{IntoResponse, Response};
    use http::StatusCode;
    use http_body_util::{combinators::Collect, BodyExt};
    use std::string::FromUtf8Error;

    pin_project_lite::pin_project! {
        /// Future returned from [`String`] implementation of [`FromRequest`]
        pub struct StringFuture {
            #[pin]
            inner: Collect<Body>,
        }
    }

    impl StringFuture {
        pub fn new(inner: Body) -> Self {
            Self { inner: inner.collect() }
        }
    }

    impl Future for StringFuture {
        type Output = Result<String, StringFutureError>;

        fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            match self.project().inner.poll(cx) {
                Ready(Ok(ok)) => match String::from_utf8(ok.to_bytes().into()) {
                    Ok(ok) => Ready(Ok(ok)),
                    Err(err) => Ready(Err(StringFutureError::Utf8(err))),
                },
                Ready(Err(err)) => Ready(Err(StringFutureError::Hyper(err))),
                Pending => Pending
            }
        }
    }

    /// Error that can be returned by [`StringFuture`]
    #[derive(Debug)]
    pub enum StringFutureError {
        Hyper(hyper::Error),
        Utf8(FromUtf8Error),
    }

    impl IntoResponse for StringFutureError {
        fn into_response(self) -> Response {
            (StatusCode::BAD_REQUEST,self.to_string()).into_response()
        }
    }

    impl std::error::Error for StringFutureError { }
    impl std::fmt::Display for StringFutureError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Hyper(hyper) => hyper.fmt(f),
                Self::Utf8(utf8) => utf8.fmt(f),
            }
        }
    }
}


