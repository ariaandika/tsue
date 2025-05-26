use hyper::body::Incoming;

use crate::{body::Body, request::Request, service::HttpService};

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
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        self.inner.call(req.map(Body::new))
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

