use std::{pin::Pin, task::{ready, Context, Poll}};

pin_project_lite::pin_project! {
    /// Map an output of a future.
    #[derive(Debug)]
    pub struct Map<F,M> {
        #[pin]
        f: F,
        m: Option<M>,
    }
}

impl<F, M> Map<F, M> {
    /// Map an output of a future.
    pub fn new<O>(f: F, m: M) -> Self
    where
        F: Future,
        M: FnOnce(F::Output) -> O,
    {
        Self { f, m: Some(m) }
    }
}

impl<F,M,O> Future for Map<F,M>
where
    F: Future,
    M: FnOnce(F::Output) -> O,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.project();
        let ok = ready!(me.f.poll(cx));
        Poll::Ready(me.m.take().expect("poll after complete")(ok))
    }
}

