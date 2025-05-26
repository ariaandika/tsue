use futures_util::{
    FutureExt,
    future::{Either, Map},
};
use http::Method;

use super::{
    handler::HandlerService,
    matcher::{Matcher, RequestInternal},
};
use crate::{
    helper::Either as Either2,
    request::Request,
    response::Response,
    service::{HttpService, Service, StatusService},
};

type MethodNotAllowed = StatusService;

/// Service that match request and delegate to either service.
///
/// user typically does not interact with this directly,
/// instead use [`route`] method, or [`get`] or [`post`] function
///
/// [`route`]: super::Router::route
pub struct Branch<S, F> {
    method: Option<Method>,
    path: Option<&'static str>,
    inner: S,
    fallback: F,
}

impl<S, F> Branch<S, F> {
    pub fn new(matcher: impl Matcher, inner: S, fallback: F) -> Self {
        let (method,path) = matcher.matcher();
        Self { method, path, inner, fallback }
    }
}

macro_rules! fn_router {
    ($name:ident $method:ident $doc:literal) => {
        #[doc = $doc]
        pub fn $name<F, S>(f: F) -> Branch<HandlerService<F, S>, MethodNotAllowed> {
            Branch {
                method: Some(Method::$method),
                path: None,
                inner: HandlerService::new(f),
                fallback: StatusService(http::StatusCode::METHOD_NOT_ALLOWED),
            }
        }
        impl<S, F> Branch<S, F> {
            #[doc = $doc]
            pub fn $name<S2, F2>(self, f: F2) -> Branch<HandlerService<F2, S2>, Branch<S, F>> {
                Branch {
                    method: Some(Method::$method),
                    path: None,
                    inner: HandlerService::new(f),
                    fallback: self,
                }
            }
        }
    };
}

fn_router!(get GET "Setup GET service.");
fn_router!(post POST "Setup POST service.");
fn_router!(put PUT "Setup PUT service.");
fn_router!(patch PATCH "Setup PATCH service.");
fn_router!(delete DELETE "Setup DELETE service.");

// ===== Service =====

impl<S, F> Service<Request> for Branch<S, F>
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
        if matcher(&self.method, self.path, &req) {
            Either::Left(self.inner.call(req).map(|e| e.map_err(Either2::Left)))
        } else {
            Either::Right(self.fallback.call(req).map(|e| e.map_err(Either2::Right)))
        }
    }
}

fn matcher(method: &Option<Method>, path: Option<&'static str>, req: &Request) -> bool {
    if let Some(method) = method {
        if method != req.method() {
            return false;
        }
    }
    if let Some(path) = path {
        if path != req.match_path() {
            return false;
        }
    }
    true
}

