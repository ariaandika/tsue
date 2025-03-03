//! the [`FromRequest`] and [`FromRequestParts`] trait
use super::{into_response::IntoResponse, ReqBody};
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

from_request! {
    Bytes,
    Error = hyper::Error;
    Future = BodyFuture;
    (_, body) => BodyFuture::new(body.collect())
}

#[doc(inline)]
pub use body_future::BodyFuture;

mod body_future {
    use super::*;
    use bytes::Bytes;
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
        pub(crate) fn new(inner: Collect<ReqBody>) -> BodyFuture {
            Self { inner }
        }
    }

    impl Future for BodyFuture {
        type Output = Result<Bytes, hyper::Error>;

        fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            match self.project().inner.poll(cx) {
                Ready(Ok(ok)) => Ready(Ok(ok.to_bytes())),
                Ready(Err(err)) => Ready(Err(err)),
                Pending => Pending
            }
        }
    }
}

impl IntoResponse for hyper::Error {
    fn into_response(self) -> super::Response {
        (
            http::StatusCode::BAD_REQUEST,
            self.to_string(),
        ).into_response()
    }
}
