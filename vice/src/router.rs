//! the [`Router`] service and supporting types
//!
//! # Usage
//!
//! [`vice::listen`] accept specific constrained [`Service`]
//!
//! using [`Router`]
//!
//! ```
//! let router = Router::new()
//!     .route("/", get(handle));
//!
//! vice::listen("0.0.0.0:3000", router).await;
//! ```
//!
//! using function handler helper like [`get`]
//!
//! ```
//! let router = get(handle);
//!
//! vice::listen("0.0.0.0:3000", router).await;
//! ```
//!
//! [`vice::listen`]: crate::listen
use crate::error::{NoMethod, NotFound};
use crate::http::{IntoResponse, Request, Response};
use futures_util::FutureExt;
use futures_util::{
    future::{self, MapErr, MapOk},
    TryFutureExt,
};
use handle::Handle;
use http::Method;
use std::future::Future;
use std::task::{Context, Poll};
use tower::{
    service_fn,
    util::{Oneshot, ServiceFn},
    Service, ServiceExt,
};

pub mod handle;

/// routes builder
///
/// # Service
///
/// implement service that satisfies [`serve`] entry point
///
/// [`serve`]: crate::runtime::serve
#[derive(Clone)]
pub struct Router<S>(S);

impl Router<NotFound> {
    /// create new router with fallback of 404
    pub fn new() -> Self {
        Self(NotFound)
    }
}

impl Default for Router<NotFound> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Router<S> {
    /// create new router with specified fallback
    pub fn new_with_fallback(fallback: S) -> Self {
        Self(fallback)
    }

    /// register new route
    pub fn route<S2>(self, path: &'static str, route: S2) -> Router<RouteBranch<S2, S>> {
        Router(RouteBranch::new(path, route, self.0))
    }
}

impl<S> Service<Request> for Router<S>
where
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.0.call(req)
    }
}

//
// NOTE: RouteBranch
//

/// branch route by request path
///
/// user typically does not interact with this directly, instead use the [`Router::route`] method
///
/// # Service
///
/// `poll_ready` will always return immediately
///
/// inner service `poll_ready` will be called in service `call` after branching
#[derive(Clone)]
pub struct RouteBranch<S,F> {
    path: &'static str,
    route: S,
    fallback: F,
}

impl<S,F> RouteBranch<S,F> {
    fn new(path: &'static str, route: S, fallback: F) -> Self {
        Self { path, route, fallback }
    }
}

impl<S,F> Service<Request> for RouteBranch<S,F>
where
    S: Service<Request> + Clone,
    F: Service<Request> + Clone,
    S::Response: IntoResponse,
    F::Response: IntoResponse,
    S::Error: IntoResponse,
    F::Error: IntoResponse,
{
    type Response = Response;
    type Error = Response;
    type Future = future::Either<
        MapErr<MapOk<Oneshot<S, Request>, fn(S::Response) -> Response>, fn(S::Error) -> Response>,
        MapErr<MapOk<Oneshot<F, Request>, fn(F::Response) -> Response>, fn(F::Error) -> Response>,
    >;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req.uri().path() == self.path {
            true => future::Either::Left(
                self.route
                    .clone()
                    .oneshot(req)
                    .map_ok(IntoResponse::into_response as _)
                    .map_err(IntoResponse::into_response)
            ),
            false => future::Either::Right(
                self.fallback
                    .clone()
                    .oneshot(req)
                    .map_ok(IntoResponse::into_response as _)
                    .map_err(IntoResponse::into_response)
            ),
        }
    }
}

//
// NOTE: Route
//

pub fn get<F,H>(f: F) -> RouteMethod<F,NoMethod,H> where F: Handle<H> + Copy {
    RouteMethod { method: Method::GET, route: f, fallback: NoMethod, _handle: std::marker::PhantomData }
}

// pub fn post<F,H>(f: F) where F: Handle<H> {
//     let mut app = HandleService { service: f };
//     let _ = Service::<Request>::call(&mut app, todo!());
// }

// pub struct HandleService<F> {
//     service: F,
// }

// impl<F,H> Service<Request> for HandleService<F>
// where
//     F: Handle<H>
// {
//     type Response = Response;
//     type Error = Infallible;
//     type Future = <F as Handle<H>>::;
//
//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }
//
//     fn call(&mut self, req: Request) -> Self::Future {
//         todo!()
//     }
// }

/// route which branch by request method
///
/// user typically does not interact with this directly,
/// instead use the available routin function like [`get`]
///
/// # Service
///
/// `poll_ready` will always return immediately
///
/// inner service `poll_ready` will be called in service `call` after branching
pub struct RouteMethod<S,F,H> {
    method: Method,
    route: S,
    fallback: F,
    _handle: std::marker::PhantomData<H>,
}

impl<S,F,H> Clone for RouteMethod<S,F,H> {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl<S,F,H> Service<Request> for RouteMethod<S,F,H>
where
    S: Handle<H> + Send + 'static,
    S::Future: Send + 'static,
    F: Service<Request> + Clone + Send + 'static,
    F::Response: IntoResponse,
    F::Error: IntoResponse,
    F::Future: Send + 'static,
{
    type Response = Response;
    type Error = Response;
    type Future = future::Either<
        future::Map<
            S::Future,
            fn(Response) -> Result<Response, Response>,
        >,
        future::MapErr<
            MapOk<Oneshot<F, Request>, fn(F::Response) -> Response>,
            fn(F::Error) -> Response,
        >,
    >;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req.method() == self.method {
            true => future::Either::Left(self.route.call(req).map(Ok as _)),
            false => future::Either::Right(
                self.fallback
                    .clone()
                    .oneshot(req)
                    .map_ok(IntoResponse::into_response as _)
                    .map_err(IntoResponse::into_response),
            ),
        }
    }
}

//
// NOTE: Fallback
//

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let route = Router::new()
            .route("/", get(page));

        // assert_type(route);
    }

    fn assert_type<S>(_: S) where S: Service<Request, Response = Response, Error = Response> + Clone, { }
    async fn page(_: Request) -> Result<Response, Response> { todo!() }
}

