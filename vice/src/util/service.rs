//! service utility types
use crate::http::{into_response::IntoResponse, Request, Response};
use hyper::service::Service;
use std::convert::Infallible;

use super::{futures::EitherInto, Either};

impl<Req,Res,Er,L,R> Service<Req> for Either<L,R>
where
    L: Service<Req, Response = Res, Error = Er>,
    R: Service<Req, Response = Res, Error = Er>,
{
    type Response = Res;
    type Error = Er;
    type Future = EitherInto<L::Future,R::Future,Result<Res,Er>>;

    fn call(&self, req: Req) -> Self::Future {
        match self {
            Either::Left(l) => Either::Left(l.call(req)).await_into(),
            Either::Right(r) => Either::Right(r.call(req)).await_into(),
        }
    }
}

macro_rules! status_service {
    ($(#[$inner:meta])* $name:ident $status:ident) => {
        $(#[$inner])*
        #[derive(Clone)]
        pub struct $name;

        impl Service<Request> for $name {
            type Response = Response;
            type Error = Infallible;
            type Future = std::future::Ready<Result<Response,Infallible>>;

            fn call(&self, _: Request) -> Self::Future {
                std::future::ready(Ok(http::StatusCode::$status.into_response()))
            }
        }
    };
}

status_service! {
    /// service that return 404 Not Found
    NotFound NOT_FOUND
}

status_service! {
    /// service that return 405 Method Not Alowed
    MethodNotAllowed METHOD_NOT_ALLOWED
}

