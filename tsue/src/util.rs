//! utility types
pub mod response;
pub mod service;

/// service which holds another service
pub trait Layer<S> {
    type Service;
    fn layer(self, service: S) -> Self::Service;
}

/// represent two type that implement the same trait
pub enum Either<L,R> {
    Left(L),
    Right(R),
}

