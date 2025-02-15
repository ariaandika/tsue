use std::{future::Future, task::Poll};

pub mod connection;

pub trait Service<Request> {
    type Response;

    type Error;

    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request) -> Self::Future;
}

impl<T,Request> Service<Request> for &mut T
where
    T: Service<Request>,
{
    type Response = T::Response;
    type Error = T::Error;
    type Future = T::Future;

    fn call(&mut self, request: Request) -> Self::Future {
        <T as Service<Request>>::call(*self, request)
    }
}

impl<T,Request> Service<Request> for Box<T>
where
    T: Service<Request>,
{
    type Response = T::Response;
    type Error = T::Error;
    type Future = T::Future;

    fn call(&mut self, request: Request) -> Self::Future {
        <T as Service<Request>>::call(&mut *self, request)
    }
}



// Util

pub fn service_fn<F,Req,Res,Err,Fut>(f: F) -> ServiceFn<F>
where
    F: Fn(Req) -> Fut,
    Fut: Future<Output = Result<Res,Err>>
{
    ServiceFn { f }
}

pub struct ServiceFn<F> {
    f: F
}

impl<F,Req,Res,Err,Fut> Service<Req> for ServiceFn<F>
where
    F: Fn(Req) -> Fut,
    Fut: Future<Output = Result<Res,Err>>
{
    type Response = Res;
    type Error = Err;
    type Future = Fut;

    fn call(&mut self, request: Req) -> Self::Future {
        (self.f)(request)
    }
}

