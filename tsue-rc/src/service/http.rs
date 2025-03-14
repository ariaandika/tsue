//! service utility types
use super::Service;
use crate::{
    futures::{EitherInto, FutureExt},
    helpers::Either,
    http::StatusCode,
    request::Request,
    response::{IntoResponse, Response},
};
use std::convert::Infallible;

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
            Either::Left(l) => l.call(req).left_into(),
            Either::Right(r) => r.call(req).right_into(),
        }
    }
}

macro_rules! status_service {
    ($doc:literal $name:ident $status:ident) => {
        #[derive(Clone)]
        #[doc = $doc]
        pub struct $name;

        impl Service<Request> for $name {
            type Response = Response;
            type Error = Infallible;
            type Future = std::future::Ready<Result<Response,Infallible>>;

            fn call(&self, _: Request) -> Self::Future {
                std::future::ready(Ok(StatusCode::$status.into_response()))
            }
        }
    };
}

status_service!("service 404 Not Found" NotFound NOT_FOUND);
status_service!("service 405 Method Not Alowed" MethodNotAllowed METHOD_NOT_ALLOWED);
