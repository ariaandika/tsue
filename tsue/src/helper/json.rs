use std::fmt;
use bytes::Bytes;
use futures_util::{FutureExt, future::Map};
use http::StatusCode;
use serde::{de::DeserializeOwned, Serialize};

use super::Json;
use crate::{
    request::{FromRequest, Request},
    response::{IntoResponse, Response},
};

// ===== FromRequest =====

type BytesFutureError = <Bytes as FromRequest>::Error;

type JsonFuture<T> = Map<
    <Bytes as FromRequest>::Future,
    fn(Result<Bytes, BytesFutureError>) -> Result<Json<T>, JsonFutureError>,
>;

impl<T: DeserializeOwned> FromRequest for Json<T> {
    type Error = JsonFutureError;
    type Future = JsonFuture<T>;

    fn from_request(req: Request) -> Self::Future {
        Bytes::from_request(req).map(|e| Ok(Json(serde_json::from_slice(&e?)?)))
    }
}

// ===== IntoResponse =====

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(ok) => (("content-type", "application/json"), ok).into_response(),
            Err(_err) => {
                #[cfg(feature = "log")]
                log::error!("failed to serialize json response: {_err}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ===== Error =====

/// Error that can be returned from [`Json`] implementation of [`FromRequest`].
#[derive(Debug)]
pub enum JsonFutureError {
    Hyper(hyper::Error),
    Serde(serde_json::Error),
}

impl From<BytesFutureError> for JsonFutureError {
    fn from(e: BytesFutureError) -> Self {
        Self::Hyper(e.into())
    }
}
impl From<serde_json::Error> for JsonFutureError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
    }
}

impl IntoResponse for JsonFutureError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

impl std::error::Error for JsonFutureError {}

impl fmt::Display for JsonFutureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Hyper(hyper) => hyper.fmt(f),
            Self::Serde(serde) => serde.fmt(f),
        }
    }
}
