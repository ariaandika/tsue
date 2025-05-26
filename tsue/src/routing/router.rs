use super::{Branch, State, matcher::Matcher, nest::Nest};
use crate::{
    request::Request,
    response::Response,
    service::{HttpService, Layer, Service, StatusService},
};

type NotFound = StatusService;

/// Routes builder.
pub struct Router<S> {
    inner: S,
}

impl Router<NotFound> {
    /// Create new `Router`.
    pub fn new() -> Router<NotFound> {
        Router { inner: StatusService(http::StatusCode::NOT_FOUND) }
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
    pub fn route<R>(self, matcher: impl Matcher, route: R) -> Router<Branch<R, S>> {
        Router {
            inner: Branch::new(matcher, route, self.inner),
        }
    }

    /// Nest another router.
    ///
    /// Nested `prefix` should starts with "/".
    ///
    /// # Panics
    ///
    /// This function will panic if `prefix` is not starts with "/".
    pub fn nest<R>(self, prefix: &'static str, route: R) -> Router<Nest<R, S>> {
        Router {
            inner: Nest::new(prefix, route, self.inner),
        }
    }

    /// Add shared state.
    pub fn state<T>(self, state: T) -> Router<State<T, S>> {
        Router {
            inner: State::new(state, self.inner),
        }
    }
}

impl<S> Service<Request> for Router<S>
where
    S: HttpService,
{
    type Response = Response;
    type Error = S::Error;
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
