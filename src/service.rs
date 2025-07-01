use std::convert::Infallible;
use tcio::futures::{Map, map};

// ===== Service =====

pub trait Service<Request> {
    type Response;

    type Error;

    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    fn call(&self, request: Request) -> Self::Future;
}

// ===== FromFn =====

pub fn from_fn<F>(f: F) -> FromFn<F> {
    FromFn { f }
}

#[derive(Debug)]
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

    fn call(&self, request: Req) -> Self::Future {
        map((self.f)(request), Ok)
    }
}
