//! Multiple [`FromRequest`] implementation helpers
//!
//! [`FromRequest`]: crate::request::FromRequest
use crate::{request::FromRequestParts, response::{Response, IntoResponse}};
use http::{request, StatusCode};
use log::error;
use std::future::{ready, Ready};

/// Extract shared state
#[derive(Clone)]
pub struct State<T>(pub T);

impl<T> FromRequestParts for State<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = Response;
    type Future = Ready<Result<Self,Self::Error>>;

    fn from_request_parts(parts: &mut request::Parts) -> Self::Future {
        ready(match parts.extensions.get::<T>().cloned() {
            Some(ok) => Ok(Self(ok)),
            None => {
                error!("State is not declared");
                Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            },
        })
    }
}

