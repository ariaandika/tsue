use crate::{
    request::Request,
    response::{IntoResponse, Response},
};
use std::convert::Infallible;

pub use hyper::service::Service;

/// a service that accept http request and return http response
pub trait HttpService:
    Service<
        Request,
        Response = Response,
        Error = Infallible,
        Future = Self::HttpFuture,
    > + Send
    + Sync
    + 'static
{
    type HttpFuture: Future<Output = Result<Response,Infallible>> + Send + Sync + 'static;
}

impl<S> HttpService for S
where
    S: Service<Request, Response = Response, Error = Infallible> + Send + Sync + 'static,
    S::Future: Send + Sync + 'static,
{
    type HttpFuture = Self::Future;
}

/// service which holds another service
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

status_service!("service 404 Not Found" NotFound NOT_FOUND);
status_service!("service 405 Method Not Alowed" MethodNotAllowed METHOD_NOT_ALLOWED);

