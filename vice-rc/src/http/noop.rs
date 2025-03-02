//! noop service
use super::{Request, Response};
use crate::service::Service;
use std::{convert::Infallible, pin::Pin};

/// noop servcice, used for debugging
#[derive(Clone)]
pub struct Noop;

impl Service<Request> for Noop {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn call(&self, _: Request) -> Self::Future {
        Box::pin(async move { Ok(Response::default()) })
    }
}

