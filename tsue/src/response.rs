//! HTTP response
use bytes::Bytes;
use http_body_util::Full;

mod into_response;

pub use http::response::Parts;

/// Represents an HTTP response
pub type Response<T = ResBody> = hyper::http::Response<T>;

/// Represents a response body
pub type ResBody = Full<Bytes>;

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

/// Response with `Content-Type` of `text/html; charset=utf-8`
pub struct Html<T>(pub T);

/// Response with `Content-Type` of `application/json`
pub struct Json<T>(pub T);

