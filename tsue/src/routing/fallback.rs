use http::StatusCode;
use hyper::service::Service;
use std::{
    convert::Infallible,
    future::{Ready, ready},
};

use super::zip::Zip;
use crate::{
    request::Request,
    response::{IntoResponse, Response}, service::HttpService,
};

/// Special service to handle fallback for [`Router`][super::Router].
///
/// # `HttpService`
///
/// Implement `HttpService` that returns 404 Not Found.
///
/// # `Zip`
///
/// Implement zip that just swap with given service.
///
/// This allow router merging.
#[derive(Debug, Clone)]
pub struct Fallback;

// ===== Service =====

impl Service<Request> for Fallback {
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, _: Request) -> Self::Future {
        ready(Ok(StatusCode::NOT_FOUND.into_response()))
    }
}

// ===== Merge =====

impl Zip for Fallback {
    type Output<S: HttpService> = S;

    fn zip<S: HttpService>(self, inner: S) -> Self::Output<S> {
        inner
    }
}
