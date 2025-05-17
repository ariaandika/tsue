//! Multiple [`FromRequest`] and [`IntoResponse`] implementation.
//!
//! [`FromRequest`]: crate::request::FromRequest
mod state;
mod json;

/// Extract shared state.
#[derive(Clone)]
pub struct State<T>(pub T);

/// JSON Request and Response helper.
///
/// Response with `Content-Type` of `application/json`.
pub struct Json<T>(pub T);


