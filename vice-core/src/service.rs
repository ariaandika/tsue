//! the [`Service`] trait
use std::{future::Future, task::Poll};

pub mod from_fn;
pub use from_fn::from_fn;

pub trait Service<Request> {
    /// [`Service::call`] success result
    type Response;

    /// [`Service::call`] failed result
    type Error;

    /// [`Service::call`] future
    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    /// poll is service ready
    ///
    /// usually used for backpressuring
    ///
    /// the default implementation is always ready
    fn poll_ready(&mut self) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    /// execute the service
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

