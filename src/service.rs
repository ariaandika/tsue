//! Service trait.
use std::convert::Infallible;
use tcio::futures::{Map, map};

use crate::body::Incoming;
use crate::http::{Request, Response};

// ===== Service =====

pub trait Service<Request> {
    type Response;

    type Error;

    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    fn call(&self, request: Request) -> Self::Future;
}

pub trait HttpService:
    Service<
        Request<Incoming>,
        Response = Response<Self::ResBody>,
        Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    >
{
    type ResBody;
}

impl<S, B> HttpService for S
where
    S: Service<
        Request<Incoming>,
        Response = Response<B>,
        Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    >
{
    type ResBody = B;
}

// ===== FromFn =====

#[inline]
pub fn from_fn<F>(f: F) -> FromFn<F> {
    FromFn { f }
}

#[derive(Debug, Clone, Default)]
pub struct FromFn<F> {
    f: F,
}

impl<F, Fut, Req, Res> Service<Req> for FromFn<F>
where
    F: Fn(Req) -> Fut,
    Fut: Future<Output = Res>,
{
    type Response = Res;

    type Error = Infallible;

    type Future = Map<Fut, fn(Res) -> Result<Res, Infallible>>;

    #[inline]
    fn call(&self, request: Req) -> Self::Future {
        map((self.f)(request), Ok)
    }
}
