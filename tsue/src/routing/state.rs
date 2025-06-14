use crate::{
    request::Request,
    service::{HttpService, Service},
};

use super::zip::Zip;

/// A service that assign a shared state.
///
/// User typically does not interact with this directly,
/// instead use the [`Router::state`][super::Router::state] method.
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

impl<T, S1: Zip<S2>, S2> Zip<S2> for State<T, S1> {
    type Output = State<T, S1::Output>;

    fn zip(self, inner: S2) -> Self::Output {
        State {
            state: self.state,
            inner: self.inner.zip(inner),
        }
    }
}
