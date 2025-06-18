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
        if req.uri().path().starts_with(self.prefix) {
            Either::Left(
                self.inner
                    .call(req.with_prefixed(self.prefix))
                    .map(|e| e.map_err(Either2::Left)),
            )
        } else {
            Either::Right(self.fallback.call(req).map(|e| e.map_err(Either2::Right)))
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
