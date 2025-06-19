//! The [`Service`] trait and helpers
use std::convert::Infallible;

use crate::{
    request::Request,
    response::{IntoResponse, Response},
    routing::Zip,
};

pub use hyper::service::Service;

// ===== HttpService =====

/// A [`Service`] that accept http request and return http response.
pub trait HttpService:
    Service<
        Request,
        Response = Response,
        Error: std::error::Error + Send + Sync + 'static,
        Future: Send + Sync + 'static,
    > + Send
    + Sync
    + 'static
{
}

impl<S> HttpService for S
where
    S: Service<Request, Response = Response> + Send + Sync + 'static,
    S::Future: Send + Sync + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
}

// ===== RouterService =====

/// A [`Service`] that accept http request and return http response.
pub trait RouterService: HttpService + Zip { }

impl<S: HttpService + Zip> RouterService for S { }

// ===== Layer =====

/// A [`Service`] which holds another service.
pub trait Layer<S> {
    type Service;

    fn layer(self, service: S) -> Self::Service;
}

// ===== Helpers =====

/// [`Service`] that response with given stats code.
#[derive(Debug, Clone)]
pub struct StatusService(pub http::StatusCode);

impl Service<Request> for StatusService {
    type Response = Response;
    type Error = Infallible;
    type Future = std::future::Ready<Result<Response,Infallible>>;

    fn call(&self, _: Request) -> Self::Future {
        std::future::ready(Ok(self.0.into_response()))
    }
}

