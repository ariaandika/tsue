//! error service and response helper
use crate::{
    body::ResBody,
    http::{IntoResponse, Request, Response},
};
use http::StatusCode;
use std::{
    convert::Infallible,
    future::{ready, Ready},
    task::{Context, Poll},
};
use tower::Service;

/// convert Error into Internal Server Error and log it
pub struct InternalError(Box<dyn std::error::Error>);


impl IntoResponse for InternalError {
    fn into_response(self) -> Response {
        tracing::error!("{}",self.0);
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(<_>::from(&b""[..]))
            .unwrap()
    }
}

/// convert Error into bad request response
pub struct BadRequest(Box<dyn std::error::Error>);

impl IntoResponse for BadRequest {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(self.0.to_string().into())
            .unwrap()
    }
}

impl<E> From<E> for BadRequest
where
    E: std::error::Error + 'static
{
    fn from(value: E) -> Self {
        Self(Box::new(value))
    }
}

/// service that response 404 Not Found
#[derive(Clone)]
pub struct NotFound;

impl Service<Request> for NotFound {
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Response, Infallible>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: Request) -> Self::Future {
        ready(Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(ResBody::Empty)
            .unwrap()))
    }
}

/// service that response 405 Method Not Allowed
#[derive(Clone)]
pub struct NoMethod;

impl Service<Request> for NoMethod {
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Response, Infallible>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: Request) -> Self::Future {
        ready(Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(ResBody::Empty)
            .unwrap()))
    }
}

