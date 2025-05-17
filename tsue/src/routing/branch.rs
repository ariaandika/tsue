use futures_util::{FutureExt, future::Either};
use http::Method;
use hyper::service::Service;
use std::convert::Infallible;

use super::{Matcher, handler::HandlerService};
use crate::{
    request::Request,
    response::Response,
    service::{HttpService, MethodNotAllowed},
};

/// Service that match request and delegate to either service.
///
/// user typically does not interact with this directly,
/// instead use [`route`] method, or [`get`] or [`post`] function
///
/// [`route`]: Router::route
pub struct Branch<S, F> {
    matcher: Matcher,
    inner: S,
    fallback: F,
}

macro_rules! fn_router {
    ($name:ident $method:ident $doc:literal) => {
        #[doc = $doc]
        pub fn $name<F, S>(f: F) -> Branch<HandlerService<F, S>, MethodNotAllowed> {
            Branch {
                matcher: Method::$method.into(),
                inner: HandlerService::new(f),
                fallback: MethodNotAllowed,
            }
        }
    };
    (self $name:ident $method:ident $doc:literal) => {
        #[doc = $doc]
        pub fn $name<S2, F2>(self, f: F2) -> Branch<HandlerService<F2, S2>, Branch<S, F>> {
            Branch {
                matcher: Method::$method.into(),
                inner: HandlerService::new(f),
                fallback: self,
            }
        }
    };
}

fn_router!(get GET "Setup GET service.");
fn_router!(post POST "Setup POST service.");
fn_router!(put PUT "Setup PUT service.");
fn_router!(patch PATCH "Setup PATCH service.");
fn_router!(delete DELETE "Setup DELETE service.");

impl<S, F> Branch<S, F> {
    pub fn new(matcher: impl Into<Matcher>, inner: S, fallback: F) -> Self {
        Self { matcher: matcher.into(), inner, fallback }
    }

    fn_router!(self get GET "Add GET service.");
    fn_router!(self post POST "Add POST service.");
    fn_router!(self put PUT "Add PUT service.");
    fn_router!(self patch PATCH "Add PATCH service.");
    fn_router!(self delete DELETE "Add DELETE service.");
}

impl<S, F> Service<Request> for Branch<S, F>
where
    S: HttpService,
    F: HttpService,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Either<S::Future, F::Future>;

    fn call(&self, req: Request) -> Self::Future {
        match self.matcher == req {
            true => self.inner.call(req).left_future(),
            false => self.fallback.call(req).right_future(),
        }
    }
}
