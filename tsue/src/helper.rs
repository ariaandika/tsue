//! Multiple [`FromRequest`] and [`IntoResponse`] implementation.
//!
//! [`FromRequest`]: crate::request::FromRequest
use http::StatusCode;

mod state;
mod html;
mod redirect;
mod either;

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "form")]
mod form;

/// Extract shared state.
#[derive(Clone)]
pub struct State<T>(pub T);

/// JSON Request and Response helper.
///
/// Parse request with `Content-Type` of `application/json`.
///
/// Response with `Content-Type` of `application/json`.
#[cfg(feature = "json")]
pub struct Json<T>(pub T);

/// Form Request helper.
///
/// Parse request with `Content-Type` of `application/x-www-form-urlencoded`.
#[cfg(feature = "form")]
pub struct Form<T>(pub T);

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

/// Sum type for [`Error`][std::error::Error], [`IntoResponse`][crate::response::IntoResponse],
/// [`Debug`][std::fmt::Debug] and [`Display`][std::fmt::Display].
#[derive(Debug)]
pub enum Either<L,R> {
    Left(L),
    Right(R),
}

