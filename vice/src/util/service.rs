//! service utility types
use crate::http::{into_response::IntoResponse, Request, Response};
use hyper::service::Service;
use std::convert::Infallible;

/// service that return 404 Not Found
#[derive(Clone)]
pub struct NotFound;

impl Service<Request> for NotFound {
    type Response = Response;
    type Error = Infallible;
    type Future = std::future::Ready<Result<Response,Infallible>>;

    fn call(&self, req: Request) -> Self::Future {
        let (mut parts,_) = req.into_parts();
        std::future::ready(Ok(http::StatusCode::NOT_FOUND.into_response(&mut parts)))
    }
}

