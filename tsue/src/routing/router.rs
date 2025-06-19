use super::{
    Next, State, branch::Branch, fallback::Fallback, middleware::FromFn, middleware::from_fn,
    nest::Nest, zip::Zip,
};
use crate::{
    request::Request,
    response::{IntoResponse, Response},
    service::{HttpService, Layer, Service},
};

/// Routes builder.
#[derive(Debug)]
pub struct Router<S> {
    inner: S,
}

impl Router<Fallback> {
    /// Create new `Router`.
    pub fn new() -> Self {
        Router { inner: Fallback }
    }

    /// Create new nested `Router`.
    pub fn nested(prefix: &'static str) -> Router<Nest<Fallback, Fallback>> {
        Router {
            inner: Nest::new(prefix, Fallback, Fallback),
        }
    }
}

impl<S> Router<S> {
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

    /// Register new route.
    pub fn middleware<F, Fut>(self, f: F) -> Router<FromFn<F, S>>
    where
        F: Fn(Request, Next) -> Fut,
        Fut: Future<Output: IntoResponse>,
    {
        Router {
            inner: from_fn(f, self.inner),
        }
    }

    /// Nest another router.
    ///
    /// `prefix` should:
    ///
    /// - not be empty
    /// - not be exactly `/`
    /// - starts with `/`
    ///
    /// # Panics
    ///
    /// This function will panic if one of previous conditions are violated.
    pub fn nest<R>(self, prefix: &'static str, route: R) -> Router<Nest<R, S>> {
        Router {
            inner: Nest::new(prefix, route, self.inner),
        }
    }

    /// Merge two router.
    pub fn merge<R: HttpService>(self, router: Router<R>) -> Router<<S as Zip>::Output<R>>
    where
        S: Zip,
    {
        Router {
            inner: self.inner.zip(router.inner),
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

// ===== Zip =====

impl<S1: Zip> Zip for Router<S1> {
    type Output<S2: HttpService> = Router<S1::Output<S2>>;

    fn zip<S2: HttpService>(self, inner: S2) -> Self::Output<S2> {
        Router {
            inner: self.inner.zip(inner),
        }
    }
}

