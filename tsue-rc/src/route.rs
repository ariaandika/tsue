//! request routing
use crate::{
    futures::{EitherInto, FutureExt},
    helpers::Layer,
    http::Method,
    request::Request,
    response::Response,
    route::handler::HandlerService,
    service::{
        http::{MethodNotAllowed, NotFound},
        HttpService, Service,
    },
};
use std::convert::Infallible;

pub mod handler;

/// route builder
///
/// see [module level documentation](self) for more on routing
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

    /// layer current router service
    ///
    /// this is low level way to interact with `Router`
    ///
    /// see [`Layer`] for more information
    pub fn layer<L>(self, layer: L) -> Router<L::Service>
    where
        L: Layer<S>,
    {
        Router { inner: layer.layer(self.inner), }
    }

    /// assign new route
    pub fn route<R>(self, matcher: impl Into<Matcher>, route: R) -> Router<Branch<R, S>> {
        Router { inner: Branch {
            matcher: matcher.into(),
            inner: route,
            fallback: self.inner,
        } }
    }

    pub fn state<T>(self, state: T) -> Router<State<T, S>> {
        Router { inner: State { state, inner: self.inner } }
    }
}

impl<S> Router<S>
where
    S: HttpService
{
    /// alternative way to start server
    pub fn listen(
        self,
        addr: impl std::net::ToSocketAddrs + std::fmt::Display + Clone,
    ) -> Result<(), std::io::Error> {
        crate::listen(addr, self)
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

// ---

/// service that match request and delegate to either service
///
/// user typically does not interact with this directly,
/// instead use [`route`] method, or [`get`] or [`post`] function
///
/// [`route`]: Router::route
pub struct Branch<S,F> {
    matcher: Matcher,
    inner: S,
    fallback: F,
}

macro_rules! fn_router {
    ($name:ident $method:ident $doc:literal) => {
        #[doc = $doc]
        pub fn $name<F,S>(f: F) -> Branch<HandlerService<F,S>,MethodNotAllowed> {
            Branch {
                matcher: Method::$method.into(),
                inner: HandlerService::new(f),
                fallback: MethodNotAllowed,
            }
        }
    };
    (self $name:ident $method:ident $doc:literal) => {
        #[doc = $doc]
        pub fn $name<S2,F2>(self, f: F2) -> Branch<HandlerService<F2, S2>, Branch<S, F>> {
            Branch {
                matcher: Method::$method.into(),
                inner: HandlerService::new(f),
                fallback: self,
            }
        }
    };
}

fn_router!(get GET "setup GET service");
fn_router!(post POST "setup POST service");
fn_router!(put PUT "setup PUT service");
fn_router!(patch PATCH "setup PATCH service");
fn_router!(delete DELETE "setup DELETE service");

impl<S, F> Branch<S, F> {
    fn_router!(self get GET "add GET service");
    fn_router!(self post POST "add POST service");
    fn_router!(self put PUT "add PUT service");
    fn_router!(self patch PATCH "add PATCH service");
    fn_router!(self delete DELETE "add DELETE service");
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
        match self.matcher == req {
            true => self.inner.call(req).left_into(),
            false => self.fallback.call(req).right_into(),
        }
    }
}

// ---

/// partially match request
#[derive(Clone,Default)]
pub struct Matcher {
    path: Option<&'static str>,
    method: Option<Method>,
}

impl PartialEq<Request> for Matcher {
    fn eq(&self, other: &Request) -> bool {
        if let Some(path) = self.path {
            if other.path().eq(path) {
                return false;
            }
        }
        if let Some(method) = &self.method {
            if method != &other.method() {
                return false;
            }
        }
        true
    }
}

macro_rules! matcher_from {
    ($id:pat,$ty:ty => $($tt:tt)*) => {
        impl From<$ty> for Matcher {
            fn from($id: $ty) -> Self {
                Self $($tt)*
            }
        }
    };
}

matcher_from!(_,() => ::default());
matcher_from!(value,Method => { method: Some(value), ..Default::default() });
matcher_from!(value,&'static str => { path: Some(value), ..Default::default() });
matcher_from!((p,m),(&'static str,Method) => { path: Some(p), method: Some(m) });

// ---

pub struct State<T,S> {
    #[allow(dead_code)]
    state: T,
    inner: S,
}

impl<T, S> Service<Request> for State<T, S>
where
    T: Clone + Send + Sync + 'static,
    S: HttpService,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, req: Request) -> Self::Future {
        // TODO: req.extensions_mut().insert(self.state.clone());
        self.inner.call(req)
    }
}

