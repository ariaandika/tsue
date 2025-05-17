use http::{HeaderValue, StatusCode, header::LOCATION};

use super::Redirect;
use crate::response::{IntoResponse, Response};

impl Redirect {
    /// by default it will redirect with 307 Temporary Redirect
    pub fn new(location: impl Into<String>) -> Redirect {
        Redirect {
            status: StatusCode::TEMPORARY_REDIRECT,
            location: location.into(),
        }
    }

    /// redrect with custom status code
    pub fn with_status(status: StatusCode, location: impl Into<String>) -> Redirect {
        Redirect { status, location: location.into() }
    }
}

impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        (
            [(LOCATION, HeaderValue::from_str(&self.location).unwrap())],
            self.status,
        )
            .into_response()
    }
}
