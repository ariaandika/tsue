//! Multiple [`FromRequest`] and [`IntoResponse`] implementation.
//!
//! [`FromRequest`]: crate::request::FromRequest
use http::StatusCode;

mod state;
mod json;
mod html;
mod redirect;

/// Extract shared state.
#[derive(Clone)]
pub struct State<T>(pub T);

/// JSON Request and Response helper.
///
/// Response with `Content-Type` of `application/json`.
pub struct Json<T>(pub T);

/// HTML Response helper.
///
/// Response with `Content-Type` of `text/html; charset=utf-8`
pub struct Html<T>(pub T);

/// HTTP Redirect helper.
///
/// Response with `3xx` status code
pub struct Redirect {
    status: StatusCode,
    location: String,
}

