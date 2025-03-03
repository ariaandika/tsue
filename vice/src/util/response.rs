//! respones utility types
use crate::http::into_response::IntoResponse;


pub struct BadRequest<E>(E);

impl<E> BadRequest<E> {
    pub fn new(inner: E) -> Self {
        Self(inner)
    }
}

impl<E> From<E> for BadRequest<E>
where
    E: std::fmt::Display
{
    fn from(value: E) -> Self {
        Self(value)
    }
}

impl<E> IntoResponse for BadRequest<E>
where
    E: std::fmt::Display
{
    fn into_response(self) -> crate::http::Response {
        (
            http::StatusCode::BAD_REQUEST,
            self.0.to_string(),
        ).into_response()
    }
}

