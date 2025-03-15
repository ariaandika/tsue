//! future utility types
use crate::helpers::Either;
use std::{
    marker::PhantomData,
    task::{
        ready,
        Poll::{self, *},
    },
};

/// extension trait for `Future` trait
pub trait FutureExt: Future {
    /// map future output
    fn map<M,R>(self, mapper: M) -> Map<Self,M>
    where
        M: FnOnce(Self::Output) -> R,
        Self: Sized,
    {
        Map { inner: self, mapper: Some(mapper)  }
    }

    /// map future output into `Result<T,Infallible>`
    fn map_infallible(self) -> MapInfallible<Self>
    where
        Self: Sized
    {
        MapInfallible { inner: self }
    }

    /// map future output into `Result<T,Infallible>`
    fn and_then_or<M,L,R>(self, mapper: M) -> AndThenOr<Self,M,L>
    where
        M: FnOnce(Self::Output) -> Result<L,R>,
        L: Future<Output = R>,
        Self: Sized,
    {
        AndThenOr::First { f: self, mapper: Some(mapper) }
    }

    /// convert future into `Either` as the left variant
    fn left<R>(self) -> EitherFuture<Self,R>
    where
        R: Future,
        Self: Sized,
    {
        EitherFuture::Left { left: self }
    }

    /// convert future into `Either` as the right variant
    fn right<L>(self) -> EitherFuture<L,Self>
    where
        L: Future,
        Self: Sized,
    {
        EitherFuture::Right { right: self }
    }

    /// convert future into `Either` as the left variant
    /// where the output implement the same `Into`
    fn left_into<R,O>(self) -> EitherInto<Self,R,O>
    where
        Self::Output: Into<O>,
        R: Future,
        R::Output: Into<O>,
        Self: Sized,
    {
        EitherInto::Left { left: self, _p: PhantomData }
    }

    /// convert future into `Either` as the right variant
    /// where the output implement the same `Into`
    fn right_into<L,O>(self) -> EitherInto<L,Self,O>
    where
        Self::Output: Into<O>,
        L: Future,
        L::Output: Into<O>,
        Self: Sized,
    {
        EitherInto::Right { right: self }
    }
}

impl<F> FutureExt for F where F: Future { }

pub trait TryFutureExt: Future {
    /// map future output if its `Ok`
    fn map_ok<M,T,E,T2>(self, mapper: M) -> MapOk<Self,M>
    where
        Self: Future<Output = Result<T,E>>,
        M: FnOnce(T) -> T2,
        Self: Sized,
    {
        MapOk { inner: self, mapper: Some(mapper)  }
    }

    /// map future output if its `Err`
    fn map_err<M,T,E,E2>(self, mapper: M) -> MapErr<Self,M>
    where
        Self: Future<Output = Result<T,E>>,
        M: FnOnce(E) -> E2,
        Self: Sized,
    {
        MapErr { inner: self, mapper: Some(mapper)  }
    }
}

impl<F> TryFutureExt for F where F: Future { }

// ---

pin_project_lite::pin_project! {
    /// map the output of a future
    pub struct Map<F,M> {
        #[pin]
        inner: F,
        mapper: Option<M>,
    }
}

impl<F,M,R> Future for Map<F,M>
where
    F: Future,
    M: FnOnce(F::Output) -> R,
{
    type Output = R;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let me = self.project();
        Poll::Ready((me.mapper.take().expect("poll after complete"))(ready!(me.inner.poll(cx))))
    }
}

// ---

pin_project_lite::pin_project! {
    pub struct MapInfallible<F> {
        #[pin]
        inner: F
    }
}

impl<F> Future for MapInfallible<F>
where
    F: Future,
{
    type Output = Result<F::Output, std::convert::Infallible>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Ok(ready!(self.project().inner.poll(cx))))
    }
}

// ---

pin_project_lite::pin_project! {
    #[project = AndThenOrProj]
    pub enum AndThenOr<F,M,L> {
        First { #[pin] f: F, mapper: Option<M> },
        Second { #[pin] f: L },
    }
}

impl<F,M,L,R> Future for AndThenOr<F,M,L>
where
    F: Future,
    M: FnOnce(F::Output) -> Result<L,R>,
    L: Future<Output = R>,
{
    type Output = R;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project() {
                AndThenOrProj::First { f, mapper } => {
                    let ok = ready!(f.poll(cx));
                    match (mapper.take().expect("poll after complete"))(ok) {
                        Ok(fut2) => {
                            self.set(AndThenOr::Second { f: fut2 });
                        },
                        Err(r) => return Poll::Ready(r),
                    }
                },
                AndThenOrProj::Second { f } => return f.poll(cx),
            }
        }
    }
}

// ---

pin_project_lite::pin_project! {
    /// poll either two futures resulting in either output
    #[project = EitherProj]
    pub enum EitherFuture<L,R> {
        Left { #[pin] left: L },
        Right { #[pin] right: R },
    }
}

impl<L,R> Future for EitherFuture<L,R>
where
    L: Future,
    R: Future,
{
    type Output = Either<L::Output,R::Output>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project() {
            EitherProj::Left { left } => Poll::Ready(Either::Left(ready!(left.poll(cx)))),
            EitherProj::Right { right } => Poll::Ready(Either::Right(ready!(right.poll(cx)))),
        }
    }
}

// --

pin_project_lite::pin_project! {
    /// two futures where the output implement the same `Into`
    #[project = EitherIntoProj]
    pub enum EitherInto<L,R,O> {
        Left { #[pin] left: L, _p: PhantomData<O> },
        Right { #[pin] right: R },
    }
}

impl<L,R,O> Future for EitherInto<L,R,O>
where
    L: Future,
    R: Future,
    L::Output: Into<O>,
    R::Output: Into<O>,
{
    type Output = O;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project() {
            EitherIntoProj::Left { left, .. } => Poll::Ready(ready!(left.poll(cx)).into()),
            EitherIntoProj::Right { right } => Poll::Ready(ready!(right.poll(cx)).into()),
        }
    }
}

// ---

pin_project_lite::pin_project! {
    /// map the output of a future
    pub struct MapOk<F,M> {
        #[pin]
        inner: F,
        mapper: Option<M>,
    }
}

impl<F,M,T,T2,E> Future for MapOk<F,M>
where
    F: Future<Output = Result<T,E>>,
    M: FnOnce(T) -> T2,
{
    type Output = Result<T2,E>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let me = self.project();
        match ready!(me.inner.poll(cx)) {
            Ok(ok) => Ready(Ok(me.mapper.take().expect("poll after complete")(ok))),
            Err(err) => Ready(Err(err)),
        }
    }
}

// ---

pin_project_lite::pin_project! {
    /// map the output of a future
    pub struct MapErr<F,M> {
        #[pin]
        inner: F,
        mapper: Option<M>,
    }
}

impl<F,M,T,E,E2> Future for MapErr<F,M>
where
    F: Future<Output = Result<T,E>>,
    M: FnOnce(E) -> E2,
{
    type Output = Result<T,E2>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let me = self.project();
        match ready!(me.inner.poll(cx)) {
            Ok(ok) => Ready(Ok(ok)),
            Err(err) => Ready(Err(me.mapper.take().expect("poll after complete")(err))),
        }
    }
}



