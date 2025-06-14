use super::{Branch, State, fallback::Fallback, nest::Nest, zip::Zip};
use crate::{
    request::Request,
    response::Response,
    service::{HttpService, Layer, Service},
};

/// Routes builder.
pub struct Router<S> {
    inner: S,
}

impl Router<Fallback> {
    /// Create new `Router`.
    pub fn new() -> Router<Fallback> {
        Router { inner: Fallback }
    }
}

impl<S> Router<S> {
    // NOTE: fallback requires to set a runtime flag for merge to work
    //
    // /// Create new `Router` with custom fallback instead of 404 NotFound.
    // pub fn with_fallback(fallback: S) -> Router<S> {
    //     Router { inner: fallback }
    // }

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
    pub fn route<R>(self, path: &'static str, route: R) -> Router<Branch<R, S>> {
        Router {
            inner: Branch::new(path, route, self.inner),
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

    /// Merge two router.
    pub fn merge<R>(self, inner: R) -> Router<<S as Zip<R>>::Output>
    where
        S: Zip<R>,
    {
        Router {
            inner: self.inner.zip(inner),
        }
    }

    /// Add shared state.
    pub fn state<T>(self, state: T) -> Router<State<T, S>> {
        Router {
            inner: State::new(state, self.inner),
        }
    }
}

impl Default for Router<Fallback> {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Service =====

impl<S: HttpService> Service<Request> for Router<S> {
    type Response = Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, req: Request) -> Self::Future {
        self.inner.call(req)
    }
}

// ===== Merge =====

impl<S1: Zip<S2>, S2> Zip<S2> for Router<S1> {
    type Output = Router<S1::Output>;

    fn zip(self, inner: S2) -> Self::Output {
        Router {
            inner: self.inner.zip(inner),
        }
    }
}
