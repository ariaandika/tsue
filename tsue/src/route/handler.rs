//! functional route
use futures_util::{FutureExt, future::Map};
use hyper::service::Service;
use std::{convert::Infallible, marker::PhantomData};

use crate::{
    request::{Body, FromRequest, FromRequestParts, Request},
    response::{IntoResponse, Response},
};

/// functional service
#[derive(Clone)]
pub struct HandlerService<F,S> {
    inner: F,
    _s: PhantomData<S>
}

impl<F, S> HandlerService<F, S> {
    pub fn new(inner: F) -> Self {
        Self { inner, _s: PhantomData  }
    }
}

impl<F,S> Service<Request> for HandlerService<F,S>
where
    F: Handler<S>,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Map<<F as Handler<S>>::Future, fn(Response) -> Result<Response,Infallible>>;

    fn call(&self, req: Request) -> Self::Future {
        self.inner.handle(req).map(Ok)
    }
}

/// Async function as [`Service`].
///
/// This trait exists because multiple blanket implementation on [`Service`]
/// directly for multiple function with different arguments is impossible.
pub trait Handler<S> {
    type Future: Future<Output = Response>;

    fn handle(&self, req: Request) -> Self::Future;
}

use future::{Fd, Fr, Frp, FrpCall};

impl<F,Fut> Handler<()> for F
where
    F: FnOnce() -> Fut + Clone,
    Fut: Future,
    Fut::Output: IntoResponse,
{
    type Future = Map<Fut, fn(Fut::Output) -> Response>;

    fn handle(&self, _: Request) -> Self::Future {
        self.clone()().map(IntoResponse::into_response)
    }
}

impl<F,A,Fut> Handler<(A,)> for F
where
    F: FnOnce(A) -> Fut + Clone,
    Fut: Future,
    Fut::Output: IntoResponse,
    A: FromRequest,
    A::Error: IntoResponse,
{
    type Future = Fd<FrCall<A>, fn(A, F) -> Fut, Fut, F>;

    fn handle(&self, req: Request) -> Self::Future {
        fn mapper<A,F,Fut>(a: A, inner: F) -> Fut where F: FnOnce(A) -> Fut, { inner(a) }
        Fd::new(FrCall::new(req), self.clone(), mapper)
    }
}

impl<F,A1,A,Fut> Handler<(A1,A)> for F
where
    F: FnOnce(A1,A) -> Fut + Clone,
    Fut: Future,
    Fut::Output: IntoResponse,
    A1: FromRequestParts,
    A1::Error: IntoResponse,
    A: FromRequest,
    A::Error: IntoResponse,
{
    type Future=Fd<Fr<FrpCall<A1>,A1,A>,fn((A1,A),F)->Fut,Fut,F>;

    fn handle(&self, req: Request) -> Self::Future {
        let (parts,body) = req.into_parts();
        fn mapper<A1,A,F,Fut>((a1,a): (A1,A), inner: F) -> Fut where F: FnOnce(A1,A) -> Fut, { inner(a1,a) }
        Fd::new(Fr::new(FrpCall::new(parts), body), self.clone(), mapper)
    }
}

impl<F,A1,A2,A,Fut> Handler<(A1,A2,A)> for F
where
    F: FnOnce(A1,A2,A) -> Fut + Clone,
    Fut: Future,
    Fut::Output: IntoResponse,
    A1: FromRequestParts,
    A1::Error: IntoResponse,
    A2: FromRequestParts,
    A2::Error: IntoResponse,
    A: FromRequest,
    A::Error: IntoResponse,
{
    type Future = Fd<Fr<Frp<FrpCall<A1>, A1, A2>, (A1, A2), A>, fn(((A1, A2), A), F) -> Fut, Fut, F>;

    fn handle(&self, req: Request) -> Self::Future {
        let (parts,body) = req.into_parts();
        fn mapper<A1,A2,A,F,Fut>(((a1,a2),a): ((A1,A2),A), inner: F) -> Fut
        where F: FnOnce(A1,A2,A) -> Fut, { inner(a1,a2,a) }
        Fd::new(Fr::new(Frp::new(FrpCall::new(parts)), body), self.clone(), mapper)
    }
}

impl<F,A1,A2,A3,A,Fut> Handler<(A1,A2,A3,A)> for F
where
    F: FnOnce(A1,A2,A3,A) -> Fut + Clone,
    Fut: Future,
    Fut::Output: IntoResponse,
    A1: FromRequestParts,
    A1::Error: IntoResponse,
    A2: FromRequestParts,
    A2::Error: IntoResponse,
    A3: FromRequestParts,
    A3::Error: IntoResponse,
    A: FromRequest,
    A::Error: IntoResponse,
{
    type Future=Fd<Fr<Frp<Frp<FrpCall<A1>,A1,A2>,(A1,A2),A3>,((A1,A2),A3),A>,fn((((A1,A2),A3),A),F)->Fut,Fut,F>;

    fn handle(&self, req: Request) -> Self::Future {
        let (parts,body) = req.into_parts();
        fn mapper<A1,A2,A3,A,F,Fut>((((a1,a2),a3),a): (((A1,A2),A3),A), inner: F) -> Fut
        where
            F: FnOnce(A1,A2,A3,A) -> Fut,
        {
            inner(a1,a2,a3,a)
        }
        Fd::new(Fr::new(Frp::new(Frp::new(FrpCall::new(parts))), body), self.clone(), mapper)
    }
}

impl<F,A1,A2,A3,A4,A,Fut> Handler<(A1,A2,A3,A4,A)> for F
where
    F: FnOnce(A1,A2,A3,A4,A) -> Fut + Clone,
    Fut: Future,
    Fut::Output: IntoResponse,
    A1: FromRequestParts,
    A1::Error: IntoResponse,
    A2: FromRequestParts,
    A2::Error: IntoResponse,
    A3: FromRequestParts,
    A3::Error: IntoResponse,
    A4: FromRequestParts,
    A4::Error: IntoResponse,
    A: FromRequest,
    A::Error: IntoResponse,
{
    type Future=Fd<Fr<Frp<Frp<Frp<FrpCall<A1>,A1,A2>,(A1,A2),A3>,((A1,A2),A3),A4>,(((A1,A2),A3),A4),A>,fn(((((A1,A2),A3),A4),A),F)->Fut,Fut,F>;

    fn handle(&self, req: Request) -> Self::Future {
        let (parts,body) = req.into_parts();
        let mapper = |((((a1,a2),a3),a4),a),inner: Self|inner(a1,a2,a3,a4,a);
        Fd::new(Fr::new(Frp::new(Frp::new(Frp::new(FrpCall::new(parts)))), body), self.clone(), mapper)
    }
}

impl<F,A1,A2,A3,A4,A5,A,Fut> Handler<(A1,A2,A3,A4,A5,A)> for F
where
    F: FnOnce(A1,A2,A3,A4,A5,A) -> Fut + Clone,
    Fut: Future,
    Fut::Output: IntoResponse,
    A1: FromRequestParts,
    A1::Error: IntoResponse,
    A2: FromRequestParts,
    A2::Error: IntoResponse,
    A3: FromRequestParts,
    A3::Error: IntoResponse,
    A4: FromRequestParts,
    A4::Error: IntoResponse,
    A5: FromRequestParts,
    A5::Error: IntoResponse,
    A: FromRequest,
    A::Error: IntoResponse,
{
    type Future=Fd<Fr<Frp<Frp<Frp<Frp<FrpCall<A1>,A1,A2>,(A1,A2),A3>,((A1,A2),A3),A4>,(((A1,A2),A3),A4),A5>,((((A1,A2),A3),A4),A5),A>,fn((((((A1,A2),A3),A4),A5),A),F)->Fut,Fut,F>;

    fn handle(&self, req: Request) -> Self::Future {
        let (parts,body) = req.into_parts();
        let mapper = |(((((a1,a2),a3),a4),a5),a),inner: Self|inner(a1,a2,a3,a4,a5,a);
        Fd::new(Fr::new(Frp::new(Frp::new(Frp::new(Frp::new(FrpCall::new(parts))))), body), self.clone(), mapper)
    }
}

mod future {
    use std::{pin::Pin, task::{ready, Context, Poll::{self, *}}};
    use http::request;
    use super::*;

    pin_project_lite::pin_project! {
        /// future that wrap FromRequestParts future
        pub struct FrpCall<Frp>
        where
            Frp: FromRequestParts,
        {
            #[pin] f: Frp::Future,
            parts: Option<request::Parts>,
        }
    }

    impl<Frp> FrpCall<Frp>
    where
        Frp: FromRequestParts,
    {
        pub fn new(mut parts: request::Parts) -> Self {
            Self { f: Frp::from_request_parts(&mut parts), parts: Some(parts) }
        }
    }

    impl<Frp> Future for FrpCall<Frp>
    where
        Frp: FromRequestParts,
        Frp::Error: IntoResponse,
    {
        type Output = Result<(request::Parts,Frp),Response>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            let me = self.project();
            match ready!(me.f.poll(cx)) {
                Ok(frp) => Ready(Ok((me.parts.take().unwrap(),frp))),
                Err(err) => Ready(Err(err.into_response())),
            }
        }
    }

    // ---

    pin_project_lite::pin_project! {
        /// future that wrap subsequent FromRequestParts future
        #[project = FrpProj]
        pub enum Frp<Fut,Frp1,Frp2>
        where
            Frp2: FromRequestParts,
        {
            Frp1 { #[pin] f: Fut, },
            Frp2 { #[pin] f: Frp2::Future, parts: Option<request::Parts>, frp1: Option<Frp1>, },
        }
    }

    impl<Fut,Frp1,Frp2> Frp<Fut,Frp1,Frp2>
    where
        Frp2: FromRequestParts,
    {
        pub fn new(f: Fut) -> Self {
            Self::Frp1 { f }
        }
    }

    impl<Fut,Frp1,Frp2> Future for Frp<Fut,Frp1,Frp2>
    where
        Fut: Future<Output = Result<(request::Parts,Frp1),Response>>,
        Frp2: FromRequestParts,
        Frp2::Error: IntoResponse,
    {
        type Output = Result<(request::Parts,(Frp1,Frp2)),Response>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            loop {
                match self.as_mut().project() {
                    FrpProj::Frp1 { f } => match ready!(f.poll(cx)) {
                        Ok((mut parts,frp1)) => self.set(Frp::Frp2 {
                            f: Frp2::from_request_parts(&mut parts),
                            frp1: Some(frp1),
                            parts: Some(parts),
                        }),
                        Err(err) => return Ready(Err(err)),
                    },
                    FrpProj::Frp2 { f, parts, frp1, } => return match ready!(f.poll(cx)) {
                        Ok(frp2) => Ready(Ok((parts.take().unwrap(),(frp1.take().unwrap(),frp2)))),
                        Err(err) => Ready(Err(err.into_response())),
                    }
                }
            }
        }
    }

    // ---

    pin_project_lite::pin_project! {
        /// future that wrap subsequent FromRequest future
        #[project = FrProj]
        pub enum Fr<Fut,Frp1,Fr1>
        where
            Fr1: FromRequest,
        {
            Frp { #[pin] f: Fut, body: Option<Body>, },
            Fr { #[pin] f: Fr1::Future, frp: Option<Frp1>, },
        }
    }

    impl<Fut,Frp1,Fr1> Fr<Fut,Frp1,Fr1>
    where
        Fut: Future<Output = Result<(request::Parts,Frp1),Response>>,
        Fr1: FromRequest,
    {
        pub fn new(f: Fut, body: Body) -> Self {
            Self::Frp { f, body: Some(body) }
        }
    }

    impl<Fut,Frp1,Fr1> Future for Fr<Fut,Frp1,Fr1>
    where
        Fut: Future<Output = Result<(request::Parts,Frp1),Response>>,
        Fr1: FromRequest,
        Fr1::Error: IntoResponse,
    {
        type Output = Result<(Frp1,Fr1),Response>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            loop {
                match self.as_mut().project() {
                    FrProj::Frp { f, body } => match ready!(f.poll(cx)) {
                        Ok((parts,frp)) => {
                            let req = Request::from_parts(parts, body.take().unwrap());
                            self.set(Fr::Fr {
                                f: Fr1::from_request(req),
                                frp: Some(frp)
                            })
                        },
                        Err(err) => return Ready(Err(err)),
                    },
                    FrProj::Fr { f, frp } => return match ready!(f.poll(cx)) {
                        Ok(fr) => Ready(Ok((frp.take().unwrap(),fr))),
                        Err(err) => Ready(Err(err.into_response())),
                    }
                }
            }
        }
    }

    // ---

    pin_project_lite::pin_project! {
        /// future that call handle with subsequent FromRequest future
        #[project = FProj]
        pub enum Fd<Fut,M,MFut,F1> {
            Fr { #[pin] f: Fut, inner: Option<F1>, mapper: Option<M>, },
            F { #[pin] f: MFut, },
        }
    }

    impl<Fr1,Fut,M,MFut,F1> Fd<Fut,M,MFut,F1>
    where
        Fut: Future<Output = Result<Fr1,Response>>,
        M: FnOnce(Fr1,F1) -> MFut + Clone,
        MFut: Future,
        MFut::Output: IntoResponse,
    {
        pub fn new(f: Fut, inner: F1, mapper: M) -> Self {
            Self::Fr { f, inner: Some(inner), mapper: Some(mapper) }
        }
    }

    impl<Fr1,Fut,M,MFut,F1> Future for Fd<Fut,M,MFut,F1>
    where
        Fut: Future<Output = Result<Fr1,Response>>,
        M: FnOnce(Fr1,F1) -> MFut,
        MFut: Future,
        MFut::Output: IntoResponse,
    {
        type Output = Response;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            loop {
                match self.as_mut().project() {
                    FProj::Fr { f, inner, mapper } => match ready!(f.poll(cx)) {
                        Ok(fr) => {
                            let inner = inner.take().unwrap();
                            let mapper = mapper.take().unwrap();
                            self.set(Fd::F { f: (mapper)(fr,inner), });
                        },
                        Err(err) => return Ready(err),
                    }
                    FProj::F { f } => return Ready(ready!(f.poll(cx)).into_response())
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Handler;
    use http::Method;

    #[test]
    fn assert_handler() {
        assert(ap0);
        assert(ap1);
        assert(ap2);
        assert(ap3);
        assert(ap4);
        assert(ap5);
        assert(ap6);
    }

    pub fn assert<F,S>(_: F) where F: Handler<S>, { }

    async fn ap0() { }
    async fn ap1(_: Method) { }
    async fn ap2(_: Method, _: String) { }
    async fn ap3(_: Method, _: Method, _: String) { }
    async fn ap4(_: Method, _: Method, _: Method, _: String) { }
    async fn ap5(_: Method, _: Method, _: Method, _: Method, _: String) { }
    async fn ap6(_: Method, _: Method, _: Method, _: Method, _: Method, _: String) { }
}

