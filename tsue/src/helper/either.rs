use std::{fmt, pin::Pin};

use super::Either;
use crate::response::{IntoResponse, Response};

impl<L: std::error::Error, R: std::error::Error> std::error::Error for Either<L, R> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Either::Left(l) => l.source(),
            Either::Right(r) => r.source(),
        }
    }
}

impl<L: fmt::Display, R: fmt::Display> fmt::Display for Either<L, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Either::Left(l) => l.fmt(f),
            Either::Right(r) => r.fmt(f),
        }
    }
}

impl<L: IntoResponse, R: IntoResponse> IntoResponse for Either<L, R> {
    fn into_response(self) -> Response {
        match self {
            Either::Left(l) => l.into_response(),
            Either::Right(r) => r.into_response(),
        }
    }
}

// ===== Project =====

enum EitherProject<'p, L, R>
where
    Either<L, R>: 'p,
{
    Left(Pin<&'p mut L>),
    Right(Pin<&'p mut R>),
}

impl<L, R> Either<L, R> {
    fn project<'p>(self: Pin<&'p mut Self>) -> EitherProject<'p, L, R> {
        // SAFETY: self is pinned
        // no `Drop`, nor manual `Unpin` implementation.
        unsafe {
            match self.get_unchecked_mut() {
                Self::Left(left) => EitherProject::Left(Pin::new_unchecked(left)),
                Self::Right(right) => EitherProject::Right(Pin::new_unchecked(right)),
            }
        }
    }
}

impl<L, R> Future for Either<L, R>
where
    L: Future,
    R: Future<Output = L::Output>,
{
    type Output = L::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        match self.project() {
            EitherProject::Left(pin) => pin.poll(cx),
            EitherProject::Right(pin) => pin.poll(cx),
        }
    }
}
