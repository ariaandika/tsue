//! the [`Router`] service and supporting types
//!
//! [`Router`] routes builder
use crate::http::{IntoResponse, Request, Response};
use crate::body::ResBody;
use futures_util::{
    future::{self, MapErr, MapOk},
    TryFutureExt,
};
use http::{Method, StatusCode};
use std::{convert::Infallible, task::{Context, Poll}};
use tower::{
    service_fn,
    util::{Oneshot, ServiceFn},
    Service, ServiceExt,
};

//
// NOTE: Router
//

/// routes builder
///
/// # Service
///
/// implement service that satisfies `serve` entry point
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
    /// register new route
    ///
    /// # Example
    ///
    /// ```
    /// Router::new()
    ///     .route("/", get(home_page))
    ///
    /// fn home_page() { }
    /// ```
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
/// user typically does not interact with this directly, instead use the [`route`] method
///
/// # Service
///
/// `poll_ready` will always return immediately
///
/// inner service `poll_ready` will be called in `call` after branching
///
/// [`route`]: Router::route
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

pub fn get<F>(f: F) -> RouteMethod<ServiceFn<F>,NoMethod> {
    RouteMethod { method: Method::GET, route: service_fn(f), fallback: NoMethod }
}

/// route which branch by request method
#[derive(Clone)]
pub struct RouteMethod<S,F> {
    method: Method,
    route: S,
    fallback: F,
}

impl<S,F> Service<Request> for RouteMethod<S,F>
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
        match req.method() == self.method {
            true => future::Either::Left(
                self.route
                    .clone()
                    .oneshot(req)
                    .map_ok(IntoResponse::into_response as _)
                    .map_err(IntoResponse::into_response),
            ),
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

/// service that response 404 Not Found
#[derive(Clone)]
pub struct NotFound;

impl Service<Request> for NotFound {
    type Response = Response;
    type Error = Infallible;
    type Future = std::future::Ready<Result<Response, Infallible>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: Request) -> Self::Future {
        std::future::ready(Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(ResBody::Empty)
            .unwrap()))
    }
}

/// service that response 405 Method Not Allowed
#[derive(Clone)]
pub struct NoMethod;

impl Service<Request> for NoMethod {
    type Response = Response;
    type Error = Infallible;
    type Future = std::future::Ready<Result<Response, Infallible>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: Request) -> Self::Future {
        std::future::ready(Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(ResBody::Empty)
            .unwrap()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let route = Router::new()
            .route("/", get(page));

        type_check(route);
    }

    fn type_check<S>(_: S)
    where
        S: Service<Request, Response = Response, Error = Response> + Clone,
    {
    }

    async fn page(_: Request) -> Result<Response, Response> {
        todo!()
    }
}

