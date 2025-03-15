//! Future utility types
use std::{marker::PhantomData, task::{ready, Poll}};
use crate::helper::Either;

/// Extension trait for `Future` trait
pub trait FutureExt: Future {
    /// Map future output
    fn map<M,R>(self, mapper: M) -> Map<Self,M>
    where
        M: FnOnce(Self::Output) -> R,
        Self: Sized,
    {
        Map { inner: self, mapper: Some(mapper)  }
    }

    /// Map future output into `Result<T,Infallible>`
    fn map_infallible(self) -> MapInfallible<Self>
    where
        Self: Sized
    {
        MapInfallible { inner: self }
    }

    /// Map future output into another future
    fn and_then<M,F2>(self, mapper: M) -> AndThen<Self,M,F2>
    where
        M: FnOnce(Self::Output) -> F2,
        F2: Future,
        Self: Sized,
    {
        AndThen::First { f: self, mapper: Some(mapper) }
    }

    /// Convert future into `Either` as the left variant
    fn left<R>(self) -> EitherFuture<Self,R>
    where
        R: Future,
        Self: Sized,
    {
        EitherFuture::Left { left: self }
    }

    /// Convert future into `Either` as the right variant
    fn right<L>(self) -> EitherFuture<L,Self>
    where
        L: Future,
        Self: Sized,
    {
        EitherFuture::Right { right: self }
    }

    /// Convert future into `Either` as the left variant
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

    /// Convert future into `Either` as the right variant
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

/// Extension trait for `Future` trait that output a `Result`
pub trait TryFutureExt: Future {
    /// Map future output if it `Result::Ok`
    fn map_ok<M,T,E,T2>(self, mapper: M) -> MapOk<Self,M>
    where
        Self: Future<Output = Result<T,E>>,
        M: FnOnce(T) -> T2,
        Self: Sized
    {
        MapOk { inner: self, mapper: Some(mapper) }
    }

    /// Map future output if it `Result::Err`
    fn map_err<M,T,E,E2>(self, mapper: M) -> MapErr<Self,M>
    where
        Self: Future<Output = Result<T,E>>,
        M: FnOnce(E) -> E2,
        Self: Sized
    {
        MapErr { inner: self, mapper: Some(mapper) }
    }
}

impl<F> TryFutureExt for F where F: Future { }

// ---

pin_project_lite::pin_project! {
    /// Map future output
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
    /// Map future output into `Result<T,Infallible>`
    pub struct MapInfallible<F> {
        #[pin]
        inner: F
    }
}

impl<F> Future for MapInfallible<F> where F: Future {
    type Output = Result<F::Output, std::convert::Infallible>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Ok(ready!(self.project().inner.poll(cx))))
    }
}

// ---

pin_project_lite::pin_project! {
    /// Map future output into another future
    #[project = AndThenProj]
    pub enum AndThen<F,M,F2> {
        First { #[pin] f: F, mapper: Option<M> },
        Second { #[pin] f2: F2 },
    }
}

impl<F,M,F2> Future for AndThen<F,M,F2>
where
    F: Future,
    M: FnOnce(F::Output) -> F2,
    F2: Future,
{
    type Output = F2::Output;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project() {
                AndThenProj::First { f, mapper } => {
                    let f2 = (mapper.take().expect("poll after complete"))(ready!(f.poll(cx)));
                    self.set(AndThen::Second { f2 });
                },
                AndThenProj::Second { f2 } => return f2.poll(cx),
            }
        }
    }
}

// ---

pin_project_lite::pin_project! {
    /// Two futures resulting in Either output
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

pin_project_lite::pin_project! {
    /// Two futures where the output implement the same `Into`
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
    /// Map future output if it `Result::Ok`
    pub struct MapOk<F,M> {
        #[pin]
        inner: F,
        mapper: Option<M>,
    }
}

impl<F,M,T,E,T2> Future for MapOk<F,M>
where
    F: Future<Output = Result<T,E>>,
    M: FnOnce(T) -> T2,
{
    type Output = Result<T2,E>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let me = self.project();
        match ready!(me.inner.poll(cx)) {
            Ok(ok) => Poll::Ready(Ok((me.mapper.take().expect("poll after complete"))(ok))),
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

// ---

pin_project_lite::pin_project! {
    /// Map future output if it `Result::Err`
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
            Ok(ok) => Poll::Ready(Ok(ok)),
            Err(err) => Poll::Ready(Err((me.mapper.take().expect("poll after complete"))(err))),
        }
    }
}

