use std::fmt;

use crate::response::{IntoResponse, Response};

use super::Either;

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

