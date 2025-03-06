//! utility types
pub mod futures;
pub mod response;
pub mod service;

#[doc(inline)]
pub use futures::FutureExt;

/// represent two type that implement the same trait
pub enum Either<L,R> {
    Left(L),
    Right(R),
}

