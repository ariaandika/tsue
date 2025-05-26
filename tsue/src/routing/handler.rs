//! functional route
use futures_util::{FutureExt, future::Map};
use std::{convert::Infallible, marker::PhantomData};

use crate::{
    request::{FromRequest, FromRequestParts, Request},
    response::{IntoResponse, Response},
    service::Service,
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

impl<F, A, Fut> Handler<(A,)> for F
where
    F: FnOnce(A) -> Fut + Clone,
    Fut: Future,
    Fut::Output: IntoResponse,
    A: FromRequest,
    A::Error: IntoResponse,
{
    type Future = Merge<<(A,) as FromRequest>::Future, Fut, F, fn(F, (A,)) -> Fut>;

    fn handle(&self, req: Request) -> Self::Future {
        merge(<(A,)>::from_request(req), self.clone(), |s,(a1,)|s(a1))
    }
}

macro_rules! foo {
    ($($r:ident,)*) => {
        impl<F,$($r,)*A,Fut> Handler<($($r,)*A)> for F
        where
            F: FnOnce($($r,)*A) -> Fut + Clone,
            Fut: Future,
            Fut::Output: IntoResponse,
            $(
                $r: FromRequestParts,
                $r::Error: IntoResponse,
            )*
            A: FromRequest,
            A::Error: IntoResponse,
        {
            type Future = Merge<<($($r,)*A) as FromRequest>::Future, Fut, F, fn(F, ($($r,)*A)) -> Fut>;

            fn handle(&self, req: Request) -> Self::Future {
                #[allow(non_snake_case)]
                merge(<($($r,)*A)>::from_request(req), self.clone(), |s,($($r,)*a)|s($($r,)*a))
            }
        }
    };
}

foo!(A1,);
foo!(A1,A2,);
foo!(A1,A2,A3,);
foo!(A1,A2,A3,A4,);
foo!(A1,A2,A3,A4,A5,);
foo!(A1,A2,A3,A4,A5,A6,);
foo!(A1,A2,A3,A4,A5,A6,A7,);

fn merge<F, F2, R, S, M>(f: F, s: S, m: M) -> Merge<F, F2, S, M>
where
    F: Future<Output = Result<R,Response>>,
    M: FnOnce(S,R) -> F2,
{
    Merge::P1 {
        f,
        s: Some(s),
        m: Some(m),
    }
}

pin_project_lite::pin_project! {
    #[project = MergeProj]
    pub enum Merge<F,F2,S,M> {
        P1 { #[pin] f: F, s: Option<S>, m: Option<M> },
        P2 { #[pin] f: F2 },
    }
}

impl<F, F2, R, S, M> Future for Merge<F, F2, S, M>
where
    F: Future<Output = Result<R,Response>>,
    F2: Future,
    M: FnOnce(S,R) -> F2,
    F2::Output: IntoResponse,
{
    type Output = Response;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        use std::task::ready;
        loop {
            match self.as_mut().project() {
                MergeProj::P1 { f, s, m } => {
                    let ok = match ready!(f.poll(cx)) {
                        Ok(ok) => ok,
                        Err(_) => todo!(),
                    };
                    let s = s.take().unwrap();
                    let f = m.take().unwrap()(s,ok);
                    self.as_mut().set(Self::P2 { f });
                }
                MergeProj::P2 { f } => {
                    let ok = ready!(f.poll(cx));
                    return std::task::Poll::Ready(ok.into_response());
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Handler;
    use http::Method as M;
    type S = String;

    #[test]
    fn assert_handler() {
        assert(ap0);
        assert(ap1);
        assert(ap2);
        assert(ap3);
        assert(ap4);
        assert(ap5);
        assert(ap6);
        assert(ap7);
    }

    pub fn assert<F,S>(_: F) where F: Handler<S>, { }

    async fn ap0() { }
    async fn ap1(_: M) { }
    async fn ap2(_: M, _: S) { }
    async fn ap3(_: M, _: M, _: S) { }
    async fn ap4(_: M, _: M, _: M, _: S) { }
    async fn ap5(_: M, _: M, _: M, _: M, _: S) { }
    async fn ap6(_: M, _: M, _: M, _: M, _: M, _: S) { }
    async fn ap7(_: M, _: M, _: M, _: M, _: M, _: M, _: S) { }
}

