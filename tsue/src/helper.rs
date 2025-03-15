//! Utility types

/// Represent two type that implement the same trait
pub enum Either<L,R> {
    Left(L),
    Right(R),
}

mod service {
    use crate::future::{EitherInto, FutureExt};
    use super::Either;
    use hyper::service::Service;

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
}

mod response {
    use crate::response::{IntoResponse, Response};

    use super::Either;

    impl<L,R> IntoResponse for Either<L,R>
    where
        L: IntoResponse,
        R: IntoResponse,
    {
        fn into_response(self) -> Response {
            match self {
                Either::Left(l) => l.into_response(),
                Either::Right(r) => r.into_response(),
            }
        }
    }
}

