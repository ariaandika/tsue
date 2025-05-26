use futures_util::{FutureExt, future::Map};
use http::StatusCode;
use hyper::body::Incoming;
use std::convert::Infallible;

use crate::{
    body::Body,
    request::Request,
    response::{IntoResponse, Response},
    service::HttpService,
};

/// Service adapter to allow use with [`hyper::service::Service`].
#[derive(Debug)]
pub struct Hyper<S> {
    inner: S,
}

impl<S> Hyper<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> hyper::service::Service<Request<Incoming>> for Hyper<S>
where
    S: HttpService,
    S::Error: std::error::Error,
{
    type Response = Response;
    type Error = Infallible;
    type Future =
        Map<S::Future, fn(Result<S::Response, S::Error>) -> Result<S::Response, Infallible>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        self.inner
            .call(req.map(Body::new))
            .map(|result| match result {
                Ok(ok) => Ok(ok),
                Err(_err) => {
                    #[cfg(feature = "log")]
                    log::error!("{_err}");
                    Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
                }
            })
    }
}

impl<S: HttpService> crate::service::Service<Request> for Hyper<S> {
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, req: Request) -> Self::Future {
        self.inner.call(req)
    }
}
