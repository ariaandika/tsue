//! HTTP response
use http::StatusCode;

mod into_response;

pub use into_response::Full;
pub use http::response::Parts;

/// Represents an HTTP response
pub type Response<T = ResBody> = hyper::http::Response<T>;

/// Represents a response body
pub type ResBody = Full;

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

/// Response with `3xx` status code
pub struct Redirect {
    status: StatusCode,
    location: String,
}

impl Redirect {
    /// by default it will redirect with 307 Temporary Redirect
    pub fn new(location: String) -> Redirect {
        Redirect {
            status: StatusCode::TEMPORARY_REDIRECT,
            location,
        }
    }

    /// redrect with custom status code
    pub fn with_status(status: StatusCode, location: String) -> Redirect {
        Redirect { status, location, }
    }
}

