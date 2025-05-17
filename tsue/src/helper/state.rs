use http::{StatusCode, request};
use std::future::{Ready, ready};

use super::State;
use crate::{
    request::FromRequestParts,
    response::{IntoResponse, Response},
};

impl<T> FromRequestParts for State<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = Response;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request_parts(parts: &mut request::Parts) -> Self::Future {
        ready(match parts.extensions.get::<T>().cloned() {
            Some(ok) => Ok(Self(ok)),
            None => {
                #[cfg(feature = "log")]
                log::error!("State is not declared");
                Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
        })
    }
}
