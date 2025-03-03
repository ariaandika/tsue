//! debug service
use super::{Method, Request, Response};
use crate::service::Service;
use std::{convert::Infallible, pin::Pin};

/// debug servcice, used for debugging
///
/// echo back body on POST request, otherwise 200 OK
#[derive(Clone)]
pub struct Debug;

impl Service<Request> for Debug {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn call(&self, req: Request) -> Self::Future {
        Box::pin(async move {
            match req.method() {
                Method::POST => Ok(Response::new(req.into_body().bytes().await.unwrap().into())),
                _ => Ok(Response::default()),
            }
        })
    }
}


