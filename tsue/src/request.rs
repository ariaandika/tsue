//! HTTP request
use crate::body::Body;

mod from_request;
mod tuples;

pub use http::request::Parts;
pub use from_request::StringFutureError;

/// Represents an HTTP request.
pub type Request<T = Body> = http::Request<T>;

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

/// A type that can be constructed from Request.
///
/// This trait is used as request handler parameters.
pub trait FromRequest: Sized {
    type Error;

    type Future: Future<Output = Result<Self, Self::Error>>;

    fn from_request(req: Request) -> Self::Future;
}

/// A type that can be constructed from Request parts.
///
/// This trait is used as request handler parameters.
pub trait FromRequestParts: Sized {
    type Error;

    type Future: Future<Output = Result<Self, Self::Error>>;

    fn from_request_parts(parts: &mut Parts) -> Self::Future;
}

/// Extension trait for [`Request`].
pub trait RequestExt {
    /// Create type that implement [`FromRequest`].
    fn extract<R: FromRequest>(self) -> R::Future;
}

impl RequestExt for Request {
    fn extract<R: FromRequest>(self) -> R::Future {
        R::from_request(self)
    }
}

/// Assert a type to implement [`FromRequest`].
#[doc(hidden)]
pub const fn assert_fr<T: FromRequest>() { }

/// Assert a type to implement [`FromRequestParts`].
#[doc(hidden)]
pub const fn assert_fp<T: FromRequestParts>() { }
