//! convert any async function into handler
use crate::http::{FromRequest, FromRequestPart, IntoResponse, Request, Response};
use futures_util::{future, FutureExt};
use std::{future::Future, pin::Pin};

/// any function that can be a handle
pub trait Handle<S> {
    type Future: Future<Output = Response>;
    fn call(&self, req: Request) -> Self::Future;
}

impl<F,Fut,R> Handle<()> for F
where
    F: Fn() -> Fut,
    Fut: Future<Output = R>,
    R: IntoResponse,
{
    type Future = future::Map<Fut, fn(R) -> Response>;
    fn call(&self, _: Request) -> Self::Future {
        self().map(IntoResponse::into_response)
    }
}

impl<F,A,Fut,R> Handle<(A,)> for F
where
    F: Copy + Fn(A,) -> Fut + 'static,
    A: FromRequest,
    Fut: Future<Output = R>,
    R: IntoResponse,
{
    type Future = Pin<Box<dyn Future<Output = Response>>>;
    fn call(&self, req: Request) -> Self::Future {
        let f = *self;
        Box::pin(async move {
            let a1 = match A::from_request(req).await {
                Ok(ok) => ok,
                Err(err) => return err.into_response(),
            };
            f(a1).await.into_response()
        })
    }
}

macro_rules! impl_fn {
    (@ $($b:ident,)*) => {
        impl<F,$($b,)*A,Fut,R> Handle<($($b,)*A,)> for F
        where
            F: Copy + Fn($($b,)*A,) -> Fut + 'static,
            $($b: FromRequestPart,)*
            A: FromRequest,
            Fut: Future<Output = R>,
            R: IntoResponse,
        {
            type Future = Pin<Box<dyn Future<Output = Response>>>;
            fn call(&self, req: Request) -> Self::Future {
                let f = *self;
                Box::pin(async move {
                    let (mut parts,body) = req.into_parts();
                    f(
                        $(
                            match $b::from_request_part(&mut parts).await {
                                Ok(ok) => ok,
                                Err(err) => return err.into_response(),
                            },
                        )*
                        match A::from_request(Request::from_parts(parts, body)).await {
                            Ok(ok) => ok,
                            Err(err) => return err.into_response(),
                        }
                    ).await.into_response()
                })
            }
        }
    };
    ($a:ident,) => {
        impl_fn!(@ $a,);
    };
    ($a:ident, $($b:ident,)*) => {
        impl_fn!(@ $a, $($b,)*);
        impl_fn!($($b,)*);
    };
}

impl_fn!(A1,A2,A3,A4,A5,A6,A7,);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn assert_fn() {
        app(app1);
        app(app2);
        app(app3);
        app(app4);
        app(app5);
        app(app6);
        app(app7);
        app(app8);
    }

    fn app<F,T>(_: F) where F: Handle<T> { }

    async fn app1() { }
    async fn app2(_: ()) { }
    async fn app3(_: String) { }
    async fn app4(_: (), _: String) { }
    async fn app5(_: (), _: ()) { }
    async fn app6(_: (), _: (), _: String) { }
    async fn app7(_: (), _: (), _: (), _: String) { }
    async fn app8(_: (), _: (), _: (), _: (), _: String) { }
}

