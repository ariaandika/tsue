use hyper::service::Service;
use std::convert::Infallible;

use super::{Branch, Matcher, State};
use crate::{
    request::Request,
    response::Response,
    service::{HttpService, Layer, NotFound},
};

/// Routes builder.
pub struct Router<S> {
    inner: S,
}

impl Router<NotFound> {
    /// Create new `Router`.
    pub fn new() -> Router<NotFound> {
        Router { inner: NotFound }
    }
}

impl<S> Router<S> {
    /// Create new `Router` with custom fallback instead of 404 NotFound.
    pub fn with_fallback(fallback: S) -> Router<S> {
        Router { inner: fallback }
    }

    /// Layer current router service.
    ///
    /// This is low level way to interact with `Router`.
    ///
    /// See [`Layer`] for more information.
    pub fn layer<L>(self, layer: L) -> Router<L::Service>
    where
        L: Layer<S>,
    {
        Router {
            inner: layer.layer(self.inner),
        }
    }

    /// Register new route.
    pub fn route<R>(self, matcher: impl Into<Matcher>, route: R) -> Router<Branch<R, S>> {
        Router {
            inner: Branch::new(matcher, route, self.inner),
        }
    }

    /// Add shared state.
    pub fn state<T>(self, state: T) -> Router<State<T, S>> {
        Router {
            inner: State::new(state, self.inner),
        }
    }
}

impl<S> Router<S>
where
    S: HttpService,
{
    /// Alternative way to start server
    #[cfg(feature = "tokio")]
    pub fn listen(
        self,
        addr: impl tokio::net::ToSocketAddrs + std::fmt::Display + Clone,
    ) -> impl Future<Output = Result<(), std::io::Error>> {
        crate::listen(addr, self)
    }
}

impl<S> Service<Request> for Router<S>
where
    S: HttpService,
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
