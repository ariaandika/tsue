use crate::http::{into_response::IntoResponse, Request, Response};
use hyper::service::Service;
use std::{convert::Infallible, future::Future, pin::Pin, sync::Arc};

/// route builder
///
/// # Service
///
/// this implements [`Service`] that can be used in [`listen`]
///
/// [`listen`]: crate::listen
///
/// # Example
///
/// ```
/// fn main() -> std::io::Result<()> {
///     let route = Router::new();
///     vice::listen("0.0.0.0:3000", route)
/// }
/// ```
#[derive(Clone)]
pub struct Router<S> {
    inner: Arc<S>
}

impl Router<NotFoundService> {
    /// create new 
    pub fn new() -> Router<NotFoundService> {
        Router { inner: Arc::new(NotFoundService) }
    }
}

impl<S> Router<S> {
    pub fn route<R>(self, route: R) -> Router<RouteMatch<R, S>> {
        Router {
            inner: Arc::new(RouteMatch {
                inner: route,
                fallback: Arc::into_inner(self.inner)
                    .expect("`Router` should not be cloned in builder"),
            }),
        }
    }
    pub fn route_checked<R>(self, route: R) -> Router<RouteMatch<R, S>> {
        self.route(route)
    }
}

impl<S> Service<Request> for Router<S>
where
    S: Service<Request> + Clone + Send + Sync + 'static,
    S::Response: IntoResponse + Send + 'static,
    S::Error: IntoResponse + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response,Self::Error>> + Send + 'static>>;

    fn call(&self, req: Request) -> Self::Future {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner.call(req).await.into_response())
        })
    }
}


#[derive(Clone)]
#[allow(dead_code)]
pub struct RouteMatch<S,F> {
    inner: S,
    fallback: F,
}

#[derive(Clone)]
pub struct NotFoundService;

impl Service<Request> for NotFoundService {
    type Response = Response;
    type Error = Infallible;
    type Future = std::future::Ready<Result<Response,Infallible>>;

    fn call(&self, _: Request) -> Self::Future {
        std::future::ready(Ok(http::StatusCode::NOT_FOUND.into_response()))
    }
}

