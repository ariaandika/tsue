use crate::{
    request::Request,
    service::{HttpService, Service},
};

use super::zip::Zip;

/// A service that assign a shared state.
///
/// User typically does not interact with this directly,
/// instead use the [`Router::state`][super::Router::state] method.
#[derive(Debug, Clone)]
pub struct State<T, S> {
    state: T,
    inner: S,
}

impl<T, S> State<T, S> {
    /// Create new [`State`] service.
    pub fn new(state: T, inner: S) -> Self {
        Self { state, inner }
    }
}

// ===== Service =====

impl<T: Clone + Send + Sync + 'static, S: HttpService> Service<Request> for State<T, S> {
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, mut req: Request) -> Self::Future {
        req.extensions_mut().insert(self.state.clone());
        self.inner.call(req)
    }
}

// ===== Merge =====

impl<T: Clone + Send + Sync + 'static, S1: Zip> Zip for State<T, S1> {
    type Output<S2: HttpService> = State<T, S1::Output<S2>>;

    fn zip<S2: HttpService>(self, inner: S2) -> Self::Output<S2> {
        State {
            state: self.state,
            inner: self.inner.zip(inner),
        }
    }
}
