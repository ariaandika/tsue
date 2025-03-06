//! request routing
//!
//! # Example
//!
//! ```no_run
//! use vice::router::{Router, get};
//!
//! fn main() -> std::io::Result<()> {
//!     let route = Router::new()
//!         .route("/", get(index));
//!     vice::listen("0.0.0.0:3000", route)
//! }
//!
//! async fn index() -> &'static str {
//!     "Vice Dev"
//! }
//! ```
use crate::{
    http::{Request, Response},
    util::{futures::EitherInto, service::{MethodNotAllowed, NotFound}, Either},
    HttpService,
};
use handler::HandlerService;
use http::Method;
use hyper::service::Service;
use std::convert::Infallible;

pub mod handler;

/// route builder
///
/// see [module level documentation](self) for more on routing
///
/// # Service
///
/// this implements [`Service`] that can be used in [`listen`](crate::listen)
///
pub struct Router<S> {
    inner: S,
}

impl Router<NotFound> {
    /// create new `Router`
    pub fn new() -> Router<NotFound> {
        Router { inner: NotFound }
    }
}

impl<S> Router<S> {
    /// create new `Router` with custom fallback instead of 404 NotFound
    pub fn with_fallback(fallback: S) -> Router<S> {
        Router { inner: fallback }
    }

    /// assign new route
    pub fn route<R>(self, path: &'static str, route: R) -> Router<Branch<R, S>> {
        Router {
            inner: Branch {
                path,
                inner: route,
                fallback: self.inner,
            },
        }
    }
}

impl<S> Service<Request> for Router<S>
where
    S: HttpService
{
    type Response = Response;
    type Error = Infallible;
    type Future = S::Future;

    fn call(&self, req: Request) -> Self::Future {
        self.inner.call(req)
    }
}

impl Default for Router<NotFound> {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! fn_router {
    ($doc:literal $name:ident $method:ident) => {
        #[doc = $doc]
        pub fn $name<F,S>(f: F) -> MethodRouter<HandlerService<F,S>,MethodNotAllowed> {
            MethodRouter { method: Method::$method, inner: HandlerService::new(f), fallback: MethodNotAllowed }
        }
    };
}

fn_router!("setup GET service" get GET);
fn_router!("setup POST service" post POST);
fn_router!("setup PUT service" put PUT);
fn_router!("setup PATCH service" patch PATCH);
fn_router!("setup DELETE service" delete DELETE);

/// service that match http method and delegate to either service
///
/// user typically does not interact with this directly,
/// instead use functions like [`get`] or [`post`]
pub struct MethodRouter<S,F> {
    method: Method,
    inner: S,
    fallback: F,
}

macro_rules! method_router {
    ($doc:literal $name:ident $method:ident) => {
        #[doc = $doc]
        pub fn $name<S2,F2>(self, f: F2) -> MethodRouter<HandlerService<F2, S2>, MethodRouter<S, F>> {
            MethodRouter { method: Method::$method, inner: HandlerService::new(f), fallback: self, }
        }
    };
}

impl<S, F> MethodRouter<S, F> {
    method_router!("add GET service" get GET);
    method_router!("add POST service" post POST);
    method_router!("add PUT service" put PUT);
    method_router!("add PATCH service" patch PATCH);
    method_router!("add DELETE service" delete DELETE);
}

impl<S,F> Service<Request> for MethodRouter<S,F>
where
    S: HttpService,
    F: HttpService,
{
    type Response = Response;
    type Error = Infallible;
    type Future = EitherInto<S::Future,F::Future,Result<Response,Infallible>>;

    fn call(&self, req: Request) -> Self::Future {
        match self.method == req.method() {
            true => Either::Left(self.inner.call(req)).await_into(),
            false => Either::Right(self.fallback.call(req)).await_into(),
        }
    }
}

/// service that match request path and delegate to either service
///
/// user typically does not interact with this directly, instead use [`route`] method
///
/// [`route`]: Router::route
pub struct Branch<S,F> {
    path: &'static str,
    inner: S,
    fallback: F,
}

impl<S,F> Service<Request> for Branch<S,F>
where
    S: HttpService,
    F: HttpService,
{
    type Response = Response;
    type Error = Infallible;
    type Future = EitherInto<S::Future,F::Future,Result<Response,Infallible>>;

    fn call(&self, req: Request) -> Self::Future {
        match self.path == req.uri().path() {
            true => Either::Left(self.inner.call(req)).await_into(),
            false => Either::Right(self.fallback.call(req)).await_into(),
        }
    }
}

