//! HTTP request
use crate::response::IntoResponse;

mod from_request;

pub use http::request::Parts;
pub use hyper::body::Incoming as Body;
pub use from_request::{BytesFuture, BytesFutureError, StringFuture, StringFutureError};

/// Represents an HTTP request
pub type Request<T = Body> = hyper::http::Request<T>;

// NOTE: Previously, `FromRequest` only accept mutable reference of `request::Parts`
// that allow `IntoResponse` access it, things get absurdly complicated realy quick
// when we have to carry around `request::Parts`, and it makes `IntoResponse`
// not portable because it require `request::Part` to call it
// For now, use something like `Responder` to build response which come from function
// argument which have access to `request::Parts`

// NOTE:
// using Pin<Box> in associated type is worth it instead of impl Future,
// because it can be referenced externally
// [issue](#63063 <https://github.com/rust-lang/rust/issues/63063>)

/// A type that can be constructed from Request
///
/// this trait is used as request handler parameters
pub trait FromRequest: Sized {
    type Error: IntoResponse;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request(req: Request) -> Self::Future;
}

/// A type that can be constructed from Request parts
///
/// this trait is used as request handler parameters
pub trait FromRequestParts: Sized {
    type Error: IntoResponse;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request_parts(parts: &mut Parts) -> Self::Future;
}

