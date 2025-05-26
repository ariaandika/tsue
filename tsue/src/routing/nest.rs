use futures_util::{FutureExt, future::Either};
use hyper::service::Service;
use std::convert::Infallible;

use super::matcher::RequestInternal;
use crate::{
    request::Request, response::Response, routing::matcher::Matched, service::HttpService,
};

#[derive(Debug)]
pub struct Nest<S, F> {
    prefix: &'static str,
    inner: S,
    fallback: F,
}

impl<S, F> Nest<S, F> {
    pub(crate) fn new(prefix: &'static str, inner: S, fallback: F) -> Self {
        assert!(prefix.starts_with("/"), "nested prefix should starts with `/`");
        Self { prefix: prefix.trim_end_matches("/"), inner, fallback, }
    }
}

impl<S, F> Service<Request> for Nest<S, F>
where
    S: HttpService,
    F: HttpService,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Either<S::Future, F::Future>;

    fn call(&self, req: Request) -> Self::Future {
        if match_path(&req, self.prefix) {
            self.inner.call(with_prefixed(req, self.prefix)).left_future()
        } else {
            self.fallback.call(req).right_future()
        }
    }
}

fn match_path(req: &Request, prefix: &'static str) -> bool {
    let path = req.match_path();

    if !path.starts_with(prefix) {
        return false;
    }

    matches!(path.as_bytes().get(prefix.len()), Some(b'/') | None)
}

fn with_prefixed(mut req: Request, prefix: &'static str) -> Request {
    let prefix_len = prefix.len().try_into().expect("prefix too large");

    match req.extensions_mut().get_mut::<Matched>() {
        Some(m) => {
            m.midpoint += prefix_len;
        }
        None => {
            req.extensions_mut().insert(Matched {
                midpoint: prefix_len,
            });
        }
    }
    req
}

