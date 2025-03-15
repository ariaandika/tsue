use bytes::Bytes;
use http_body_util::Full;

mod into_response;

pub use http::response::Parts;

/// Represents an HTTP response
pub type Response<T = ResBody> = hyper::http::Response<T>;
/// Represents a response body
pub type ResBody = Full<Bytes>;

/// Type that can be converted into response
///
/// this trait is used as request handler return type
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

/// Type that can be converted into response parts
///
/// this trait is used as request handler return type
pub trait IntoResponseParts {
    fn into_response_parts(self, parts: Parts) -> Parts;
}

