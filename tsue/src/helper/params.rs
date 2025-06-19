use serde::de::DeserializeOwned;
use std::future::{Ready, ready};

use super::macros::derefm;
use crate::{
    helper::{MatchedRoute, Params},
    request::FromRequestParts,
    response::{IntoResponse, Response},
    routing::extract::Deserializer,
};

derefm!(<T>|Params<T>| -> T);

impl<T: DeserializeOwned> FromRequestParts for Params<T> {
    type Error = Response;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request_parts(parts: &mut http::request::Parts) -> Self::Future {
        ready(
            MatchedRoute::extract(&parts.extensions)
                .map_err(<_>::into_response)
                .and_then(
                    |e| match T::deserialize(Deserializer::new(parts.uri.path(), e.0)) {
                        Ok(ok) => Ok(Self(ok)),
                        Err(err) => Err(err.into_response()),
                    },
                ),
        )
    }
}

