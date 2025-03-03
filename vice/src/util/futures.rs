use crate::http::{into_response::IntoResponse, Response};

pub trait FutureExt: Future + Sized {
    fn map<M>(self, mapper: M) -> Map<Self,M>;
    fn map_into_response<M>(self) -> MapIntoResponse<Self>;
}

pin_project_lite::pin_project! {
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

