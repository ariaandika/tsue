//! future utility types
use crate::http::{into_response::IntoResponse, Response};

/// extension trait for `Future`
pub trait FutureExt: Future {
    fn map<M,R>(self, mapper: M) -> Map<Self,M>
    where
        M: FnOnce(Self::Output) -> R,
        Self: Sized;
    fn map_into_response<M>(self) -> MapIntoResponse<Self>
    where
        Self: Sized;
}

impl<F> FutureExt for F
where
    F: Future
{
    fn map<M,R>(self, mapper: M) -> Map<Self,M>
    where
        M: FnOnce(Self::Output) -> R,
        Self: Sized
    {
        Map { inner: self, mapper: Some(mapper)  }
    }

    fn map_into_response<M>(self) -> MapIntoResponse<Self>
    where
        Self: Sized
    {
        MapIntoResponse { inner: self }
    }
}

pin_project_lite::pin_project! {
    /// map the output of a future
    pub struct Map<S,M> {
        #[pin]
        inner: S,
        mapper: Option<M>,
    }
}

impl<S,M,R> Future for Map<S,M>
where
    S: Future,
    M: FnOnce(S::Output) -> R,
{
    type Output = R;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        use std::task::Poll::*;
        let me = self.project();
        match me.inner.poll(cx) {
            Ready(ok) => Ready((me.mapper.take().expect("poll after complete"))(ok)),
            Pending => Pending,
        }
    }
}

pin_project_lite::pin_project! {
    /// map the output of a future [`IntoResponse`]
    ///
    /// [`IntoResponse`]: crate::http::IntoResponse
    pub struct MapIntoResponse<S> {
        #[pin]
        inner: S,
    }
}

impl<S> Future for MapIntoResponse<S>
where
    S: Future,
    S::Output: IntoResponse,
{
    type Output = Response;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        use std::task::Poll::*;
        match self.project().inner.poll(cx) {
            Ready(ok) => Ready(ok.into_response()),
            Pending => Pending,
        }
    }
}

