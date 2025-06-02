//! HTTP response
use crate::body::Body;

mod into_response;

pub use http::response::Parts;

/// Represents an HTTP response
pub type Response<T = Body> = http::Response<T>;

/// A type that can be converted into response
///
/// this trait is used as request handler return type
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

/// A type that can be converted into response parts
///
/// this trait is used as request handler return type
pub trait IntoResponseParts {
    fn into_response_parts(self, parts: Parts) -> Parts;
}

/// Assert a type to implement [`IntoResponse`].
#[doc(hidden)]
pub const fn assert_rs<T: IntoResponse>() { }

/// Assert a type to implement [`IntoResponseParts`].
#[doc(hidden)]
pub const fn assert_rp<T: IntoResponseParts>() { }
