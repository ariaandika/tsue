use futures_util::future::Either;
use std::convert::Infallible;

use super::{matcher::RequestInternal, zip::Zip};
use crate::{
    request::Request,
    response::Response,
    service::{HttpService, Service},
};

#[derive(Debug)]
pub struct Nest<S, F> {
    /// not empty
    /// not exactly `/`
    /// starts with `/`
    /// will not ends with `/`
    prefix: &'static str,
    inner: S,
    fallback: F,
}

impl<S, F> Nest<S, F> {
    /// prefix cannot be empty
    /// prefix cannot be exactly `/`
    /// prefix should starts with `/`
    /// ends of `/` will be trimmed
    pub(crate) fn new(prefix: &'static str, inner: S, fallback: F) -> Self {
        assert!(!prefix.is_empty(), "nested prefix cannot be empty");
        assert!(prefix.ne("/"), "nested prefix cannot be exactly `/`");
        assert!(prefix.starts_with("/"), "nested prefix should starts with `/`");

        Self { prefix: prefix.trim_end_matches("/"), inner, fallback, }
    }
}

// ===== Service =====

impl<S: HttpService, F: HttpService> Service<Request> for Nest<S, F> {
    type Response = Response;
    type Error = Infallible;
    type Future = Either<S::Future, F::Future>;

    fn call(&self, req: Request) -> Self::Future {
        if req.uri().path().starts_with(self.prefix) {
            Either::Left(self.inner.call(req.with_prefixed(self.prefix)))
        } else {
            Either::Right(self.fallback.call(req))
        }
    }
}

// ===== Merge =====

impl<S1: HttpService, F: Zip> Zip for Nest<S1, F> {
    type Output<S2: HttpService> = Nest<S1, F::Output<S2>>;

    fn zip<S2: HttpService>(self, inner: S2) -> Self::Output<S2> {
        Nest {
            prefix: self.prefix,
            inner: self.inner,
            fallback: self.fallback.zip(inner)
        }
    }
}
