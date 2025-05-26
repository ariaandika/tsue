//! The [`Service`] trait and helpers
use std::convert::Infallible;

use crate::{
    request::Request,
    response::{IntoResponse, Response},
};

// pub use hyper::service::Service;

pub trait Service<Request> {
    type Response;

    type Error;

    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    fn call(&self, req: Request) -> Self::Future;
}

/// A [`Service`] that accept http request and return http response.
pub trait HttpService:
    Service<Request, Response = Response, Error = Infallible, Future = Self::HttpFuture>
    + Send
    + Sync
    + 'static
{
    type HttpFuture: Future<Output = Result<Response, Infallible>> + Send + Sync + 'static;
}

impl<S> HttpService for S
where
    S: Service<Request, Response = Response, Error = Infallible> + Send + Sync + 'static,
    S::Future: Send + Sync + 'static,
{
    type HttpFuture = Self::Future;
}

/// A [`Service`] which holds another service.
pub trait Layer<S> {
    type Service;

    fn layer(self, service: S) -> Self::Service;
}

macro_rules! status_service {
    ($doc:literal $name:ident $status:ident) => {
        #[derive(Clone)]
        #[doc = $doc]
        pub struct $name;

        impl Service<Request> for $name {
            type Response = Response;
            type Error = Infallible;
            type Future = std::future::Ready<Result<Response,Infallible>>;

            fn call(&self, _: Request) -> Self::Future {
                std::future::ready(Ok(http::StatusCode::$status.into_response()))
            }
        }
    };
}

status_service!("[`Service`] that response with 404 Not Found" NotFound NOT_FOUND);
status_service!("[`Service`] that response with 405 Method Not Alowed" MethodNotAllowed METHOD_NOT_ALLOWED);

