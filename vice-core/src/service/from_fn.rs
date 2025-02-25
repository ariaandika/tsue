//! the [`FromFn`] service

use super::Service;

pub fn from_fn<F>(f: F) -> FromFn<F> {
    FromFn { inner: f }
}

#[derive(Clone)]
pub struct FromFn<F> {
    inner: F,
}

impl<Request,F,R,E,Fut> Service<Request> for FromFn<F>
where
    F: Fn(Request) -> Fut,
    Fut: Future<Output = Result<R,E>>,
{
    type Response = R;
    type Error = E;
    type Future = Fut;

    fn call(&mut self, request: Request) -> Self::Future {
        (self.inner)(request)
    }
}

