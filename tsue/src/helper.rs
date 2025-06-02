//! Multiple [`FromRequest`] and [`IntoResponse`] implementation.
//!
//! [`FromRequest`]: crate::request::FromRequest
//! [`IntoResponse`]: crate::response::IntoResponse
use http::StatusCode;

mod macros;
mod state;
mod html;
mod redirect;
mod either;

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "form")]
mod form;

/// Extract shared state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State<T>(pub T);

/// JSON Request and Response helper.
///
/// Parse request with `Content-Type` of `application/json`.
///
/// Response with `Content-Type` of `application/json`.
#[cfg(feature = "json")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Json<T>(pub T);

/// Form Request helper.
///
/// Parse request with `Content-Type` of `application/x-www-form-urlencoded`.
#[cfg(feature = "form")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Form<T>(pub T);

/// HTML Response helper.
///
/// Response with `Content-Type` of `text/html; charset=utf-8`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Html<T>(pub T);

impl<T> From<T> for Html<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

/// HTTP Redirect helper.
///
/// Response with `3xx` status code
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    status: StatusCode,
    location: String,
}

/// Sum type for [`Error`][1], [`Debug`][3], [`Display`][4], and [`IntoResponse`][2].
///
/// [1]: std::error::Error
/// [2]: crate::response::IntoResponse
/// [3]: std::fmt::Debug
/// [4]: std::fmt::Display
#[derive(Debug)]
pub enum Either<L,R> {
    Left(L),
    Right(R),
}

