//! utility types
pub mod futures;
pub mod response;
pub mod service;

use std::marker::PhantomData;
use futures::EitherInto;

pub use futures::FutureExt;

/// represent two type that implement the same trait
pub enum Either<L,R> {
    Left(L),
    Right(R),
}

impl<L,R> Either<L,R> {
    pub fn await_into<O>(self) -> EitherInto<L, R, O> {
        match self {
            Either::Left(left) => EitherInto::Left { left, _p: PhantomData },
            Either::Right(right) => EitherInto::Right { right },
        }
    }
}

