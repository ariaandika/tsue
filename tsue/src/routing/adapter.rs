use http::Request;
use hyper::{body::Incoming, service::Service};

use crate::{body::Body, response::Response, service::HttpService};

/// Service adapter to allow use with [`hyper::service::HttpService`].
#[derive(Debug)]
pub struct Hyper<S> {
    inner: S,
}

impl<S> Hyper<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Service<Request<Incoming>> for Hyper<S>
where
    S: HttpService,
{
    type Response = Response;

    type Error = S::Error;

    type Future = S::Future;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let (parts, body) = req.into_parts();
        self.inner.call(Request::from_parts(
            parts,
            Body::with_limit(body, 2_000_000),
        ))
    }
}
