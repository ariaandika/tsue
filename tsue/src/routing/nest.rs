use futures_util::{
    FutureExt,
    future::{Either, Map},
};

use super::{matcher::RequestInternal, zip::Zip};
use crate::{
    helper::Either as Either2,
    request::Request,
    response::Response,
    service::{HttpService, Service},
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

fn match_path(req: &Request, prefix: &'static str) -> bool {
    let path = req.matches_path();

    if !path.starts_with(prefix) {
        return false;
    }

    matches!(path.as_bytes().get(prefix.len()), Some(b'/') | None)
}

fn with_prefixed(mut req: Request, prefix: &'static str) -> Request {
    let prefix_len: u32 = prefix.len().try_into().expect("prefix too large");
    req.body_mut().shared_mut().path_offset += prefix_len;
    req
}

// ===== Service =====

impl<S, F> Service<Request> for Nest<S, F>
where
    S: HttpService,
    F: HttpService,
{
    type Response = Response;
    type Error = Either2<S::Error, F::Error>;
    type Future = Either<
        Map<
            S::Future,
            fn(Result<S::Response, S::Error>) -> Result<S::Response, Either2<S::Error, F::Error>>,
        >,
        Map<
            F::Future,
            fn(Result<F::Response, F::Error>) -> Result<F::Response, Either2<S::Error, F::Error>>,
        >,
    >;

    fn call(&self, req: Request) -> Self::Future {
        if match_path(&req, self.prefix) {
            Either::Left(
                self.inner
                    .call(with_prefixed(req, self.prefix))
                    .map(|e| e.map_err(Either2::Left)),
            )
        } else {
            Either::Right(self.fallback.call(req).map(|e| e.map_err(Either2::Right)))
        }
    }
}

// ===== Merge =====

impl<S1, F: Zip<S2>, S2> Zip<S2> for Nest<S1, F> {
    type Output = Nest<S1, F::Output>;

    fn zip(self, inner: S2) -> Self::Output {
        Nest {
            prefix: self.prefix,
            inner: self.inner,
            fallback: self.fallback.zip(inner)
        }
    }
}
