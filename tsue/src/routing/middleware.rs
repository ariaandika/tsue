use hyper::service::Service;
use std::{
    convert::Infallible,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Poll, ready},
};

use crate::{
    request::Request,
    response::{IntoResponse, Response},
    routing::Zip,
    service::HttpService,
};

/// ===== Middleware Service =====

#[derive(Debug, Clone)]
pub struct FromFn<F, S> {
    f: F,
    inner: Arc<S>,
}

impl<F, S> FromFn<F, S> {
    pub(crate) fn new(f: F, inner: S) -> Self {
        Self { f, inner: Arc::new(inner) }
    }
}

// ===== Service =====

impl<F, Fut, S> Service<Request> for FromFn<F, S>
where
    F: Fn(Request, Next) -> Fut,
    Fut: Future<Output: IntoResponse>,
    S: HttpService,
{
    type Response = Response;

    type Error = Infallible;

    type Future = FromFnFuture<S, S::Future, Fut>;

    fn call(&self, req: Request) -> Self::Future {
        let shared = Arc::new(Mutex::new(Shared::Fn));
        let next = Next { shared: shared.clone() };
        FromFnFuture {
            f: (self.f)(req, next),
            inner: self.inner.clone(),
            phase: Phase::Fn { is_pre: true },
            shared,
        }
    }
}

// ===== Zip =====

impl<F, Fut, S1> Zip for FromFn<F, S1>
where
    S1: Zip,
    F: Fn(Request, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output: IntoResponse> + Send + Sync + 'static,
{
    type Output<S2: HttpService> = FromFn<F, S1::Output<S2>>;

    fn zip<S2: HttpService>(self, inner: S2) -> Self::Output<S2> {
        FromFn {
            f: self.f,
            inner: Arc::new(Arc::into_inner(self.inner).expect("somehow cloned").zip(inner)),
        }
    }
}

// ===== Next =====

/// A signal to continue middleware chain.
#[derive(Debug)]
pub struct Next {
    shared: Arc<Mutex<Shared>>,
}

impl Next {
    /// Continue the middleware chain.
    pub fn next(self, req: Request) -> NextFuture {
        NextFuture { req: Some(req), shared: self.shared }
    }
}

#[derive(Debug)]
enum Shared {
    Fn,
    Req(Request),
    Res(Response),
    Invalid,
}

impl Shared {
    fn take(&mut self) -> Self {
        std::mem::replace(self, Shared::Invalid)
    }
}

/// A future returned by [`Next::next`];
#[derive(Debug)]
#[must_use = "future does nothing unless being polled/awaited"]
pub struct NextFuture {
    req: Option<Request>,
    shared: Arc<Mutex<Shared>>,
}

impl Future for NextFuture {
    type Output = Response;

    fn poll(self: Pin<&mut Self>, _: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();

        match me.req.take() {
            Some(req) => {
                let mut lock = me.shared.try_lock().expect("nobody locks");
                *lock = Shared::Req(req);
                Poll::Pending
            },
            None => {
                let mut lock = me.shared.try_lock().expect("nobody locks");
                match lock.take() {
                    Shared::Res(response) => Poll::Ready(response),
                    _ => panic!("[BUG] next did not resolve to Shared::Res"),
                }
            },
        }
    }
}

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[must_use]
    pub struct FromFnFuture<S, SF, F> {
        #[pin] f: F,
        inner: Arc<S>,
        #[pin]
        phase: Phase<SF>,
        shared: Arc<Mutex<Shared>>,
    }
}

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = PhaseProject]
    enum Phase<S> {
        Fn { is_pre: bool },
        Service { #[pin] f: S },
    }
}

impl<S, SF, F> Future for FromFnFuture<S, SF, F>
where
    S: HttpService<Future = SF>,
    SF: Future<Output = Result<Response,S::Error>>,
    F: Future<Output: IntoResponse>,
{
    type Output = Result<Response,Infallible>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut me = self.project();

        loop {
            use PhaseProject::*;
            match me.phase.as_mut().project() {
                Fn { is_pre } => {
                    if *is_pre {
                        match me.f.as_mut().poll(cx) {
                            Poll::Ready(response) => {
                                return Poll::Ready(Ok(response.into_response()));
                            },
                            Poll::Pending => {
                                let mut lock = me.shared.try_lock().expect("nobody locks");
                                match lock.take() {
                                    Shared::Fn => {
                                        return Poll::Pending;
                                    },
                                    Shared::Req(req) => {
                                        let f = me.inner.call(req);
                                        me.phase.set(Phase::Service { f });
                                    },
                                    Shared::Res(_) => panic!("[BUG] shared is Shared::Res before calling service"),
                                    Shared::Invalid => panic!("[BUG] invalid state readched"),
                                }
                            },
                        }
                    } else {
                        // shared is Shared::Res, next called
                        let res = ready!(me.f.as_mut().poll(cx)).into_response();
                        return Poll::Ready(Ok(res));
                    }
                },
                Service { f } => {
                    let res = ready!(Future::poll(f, cx)).into_response();
                    let mut lock = me.shared.try_lock().expect("nobody locks");
                    *lock = Shared::Res(res);
                    me.phase.set(Phase::Fn { is_pre: false });
                },
            }
        }
    }
}

