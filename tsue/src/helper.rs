//! Multiple [`FromRequest`] and [`IntoResponse`] implementation.
//!
//! [`FromRequest`]: crate::request::FromRequest
//! [`IntoResponse`]: crate::response::IntoResponse
use http::StatusCode;

mod macros;
mod util;

mod state;
mod html;
mod redirect;
mod either;

#[cfg(feature = "serde")]
mod params;

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "form")]
mod form;

#[cfg(feature = "ws")]
pub mod ws;

/// Get shared state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State<T>(pub T);

/// Get matched route path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchedRoute(pub &'static str);

/// Extract path paremeters.
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Params<T>(pub T);

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

/// HTTP Redirect helper.
///
/// Response with `3xx` status code
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    status: StatusCode,
    location: String,
}

/// WebSocket Upgrade.
#[cfg(all(feature = "tokio", feature = "ws"))]
#[derive(Debug)]
pub struct WsUpgrade {
    req: crate::request::Request,
}

#[cfg(all(feature = "tokio", feature = "ws"))]
pub use ws::WebSocket;

/// Sum type for [`Error`][1], [`Debug`][3], [`Display`][4], [`Future`], and [`IntoResponse`][2].
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

