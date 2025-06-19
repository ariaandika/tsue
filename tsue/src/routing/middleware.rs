use hyper::service::Service;
use std::convert::Infallible;

use crate::{
    futures::Map,
    request::Request,
    response::{IntoResponse, Response},
    service::HttpService,
};

#[derive(Debug, Clone)]
pub struct FromFn<F, S> {
    f: F,
    inner: S,
}

pub fn from_fn<F, S>(f: F, inner: S) -> FromFn<F, S> {
    FromFn { f, inner }
}

// ===== HttpService =====

impl<F, Fut, S> Service<Request> for FromFn<F, S>
where
    F: Fn(Request, S) -> Fut,
    Fut: Future<Output: IntoResponse>,
    S: HttpService + Clone,
{
    type Response = Response;

    type Error = Infallible;

    type Future = Map<Fut, fn(Fut::Output) -> Result<Response,Infallible>>;

    fn call(&self, req: Request) -> Self::Future {
        Map::new((self.f)(req, self.inner.clone()), |e|Ok(e.into_response()))
    }
}

// impl<F, Fut, S, Fut2> Service<Request> for FromFn<F, S>
// where
//     F: Fn(Request, S) -> Fut,
//     Fut: Future<Output: IntoResponse>,
//     S: Fn(Request) -> Fut2 + Copy,
//     Fut2: Future<Output = Response>
// {
//     type Response = Response;
//
//     type Error = Infallible;
//
//     type Future = Map<Fut, fn(Fut::Output) -> Result<Response,Infallible>>;
//
//     fn call(&self, req: Request) -> Self::Future {
//         Map::new((self.f)(req, self.inner), |e|Ok(e.into_response()))
//     }
// }

// ===== Zip =====

// impl<F, Fut, S1> Zip for FromFn<F, S1> {
// }

