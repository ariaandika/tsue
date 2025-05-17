//! HTTP response
mod body;
mod into_response;

pub use http::response::Parts;

/// Represents an HTTP response
pub type Response<T = Body> = hyper::http::Response<T>;

pub use body::Body;

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

