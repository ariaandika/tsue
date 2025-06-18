use futures_util::{
    FutureExt,
    future::{Either, Map},
};
use http::{Method, StatusCode};
use std::future::{Ready, ready};

use super::{handler::HandlerService, matcher::Path, zip::Zip};
use crate::{
    common::log,
    helper::{Either as Either2, MatchedRoute},
    request::{FromRequestParts, Request},
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
    filter: Filter,
    inner: S,
    fallback: F,
}

#[derive(Debug)]
enum Filter {
    Path(Path),
    Method(Method),
}

impl<S, F> Branch<S, F> {
    pub fn new(path: &'static str, inner: S, fallback: F) -> Self {
        Self { filter: Filter::Path(Path::new(path)), inner, fallback }
    }
}

fn_router!(get GET "Setup GET service.");
fn_router!(post POST "Setup POST service.");
fn_router!(put PUT "Setup PUT service.");
fn_router!(patch PATCH "Setup PATCH service.");
fn_router!(delete DELETE "Setup DELETE service.");

// ===== Service =====

impl<S: HttpService, F: HttpService> Service<Request> for Branch<S, F> {
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

    fn call(&self, mut req: Request) -> Self::Future {
        if match &self.filter {
            Filter::Method(method) => method == req.method(),
            Filter::Path(path) => path.matches(&req),
        } {
            if let Filter::Path(path) = &self.filter {
                req.extensions_mut().insert(MatchedRoute(path.value()));
            }
            Either::Left(self.inner.call(req).map(|e| e.map_err(Either2::Left)))
        } else {
            Either::Right(self.fallback.call(req).map(|e| e.map_err(Either2::Right)))
        }
    }
}

impl MatchedRoute {
    pub(crate) fn extract(ext: &http::Extensions) -> Result<Self, StatusCode> {
        match ext.get::<Self>() {
            Some(ok) => Ok(ok.clone()),
            None => {
                log!(
                    "failed to get route parameteer, handler probably called in non-route service"
                );
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

impl FromRequestParts for MatchedRoute {
    type Error = StatusCode;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request_parts(parts: &mut http::request::Parts) -> Self::Future {
        ready(Self::extract(&parts.extensions))
    }
}

// ===== Zip =====

impl<S1: HttpService, F: Zip> Zip for Branch<S1, F> {
    type Output<S2: HttpService> = Branch<S1, F::Output<S2>>;

    fn zip<S2: HttpService>(self, inner: S2) -> Self::Output<S2> {
        Branch {
            filter: self.filter,
            inner: self.inner,
            fallback: self.fallback.zip(inner)
        }
    }
}

// ===== Macros =====

macro_rules! fn_router {
    ($name:ident $method:ident $doc:literal) => {
        #[doc = $doc]
        pub fn $name<F: super::handler::Handler<S>, S>(f: F) -> Branch<HandlerService<F, S>, MethodNotAllowed> {
            Branch {
                filter: Filter::Method(Method::$method),
                inner: HandlerService::new(f),
                fallback: StatusService(http::StatusCode::METHOD_NOT_ALLOWED),
            }
        }
        impl<S, F> Branch<S, F> {
            #[doc = $doc]
            pub fn $name<F2: super::handler::Handler<S2>, S2>(self, f: F2) -> Branch<HandlerService<F2, S2>, Branch<S, F>> {
                Branch {
                    filter: Filter::Method(Method::$method),
                    inner: HandlerService::new(f),
                    fallback: self,
                }
            }
        }
    };
}

pub(crate) use fn_router;

