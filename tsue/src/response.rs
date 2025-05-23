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

// ===== Box Error Response =====

/// An [`std::error::Error`] and [`IntoResponse`].
pub trait ErrorResponse: std::error::Error + IntoResponse { }

impl<R: std::error::Error + IntoResponse> ErrorResponse for R { }

/// A [`Box`] of [`ErrorResponse`].
pub type BoxErrorRespone = Box<dyn ErrorResponse + Send + Sync + 'static>;

impl<R: ErrorResponse + Send + Sync + 'static> From<R> for BoxErrorRespone {
    fn from(value: R) -> Self {
        Box::new(value)
    }
}

/// A [`Result`][std::result::Result] with [`Err`] variant of [`BoxErrorResponse`].
pub type Result<T,E = BoxErrorRespone> = std::result::Result<T, E>;

