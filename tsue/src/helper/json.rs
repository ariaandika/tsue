use http::{HeaderName, HeaderValue, StatusCode, header::CONTENT_TYPE};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fmt, future::ready};

use super::{Json, macros::derefm};
use crate::{
    body::BodyError,
    common::log,
    futures::Map,
    helper::Either,
    request::{FromRequest, Request},
    response::{IntoResponse, Response},
};

derefm!(<T>|Json<T>| -> T);

fn validate(req: &Request) -> Option<()> {
    req.headers()
        .get(CONTENT_TYPE)?
        .to_str()
        .ok()?
        .eq_ignore_ascii_case("application/json")
        .then_some(())
}

// ===== FromRequest =====

type JsonMap<V = Value> =
    fn(Result<crate::body::Collected, BodyError>) -> Result<V, JsonFutureError>;

type JsonFuture<V = Value> = Either<
    Map<crate::body::Collect, JsonMap<V>>,
    std::future::Ready<Result<V, JsonFutureError>>,
>;

impl FromRequest for Value {
    type Error = JsonFutureError;
    type Future = JsonFuture;

    fn from_request(req: Request) -> Self::Future {
        match validate(&req) {
            Some(()) => Either::Left(Map::new(
                req.into_body().collect_body(),
                (|e| serde_json::from_slice(&e?.into_bytes()).map_err(Into::into)) as JsonMap,
            )),
            None => Either::Right(ready(Err(JsonFutureError::ContentType))),
        }
    }
}

impl<T: DeserializeOwned> FromRequest for Json<T> {
    type Error = JsonFutureError;
    type Future = JsonFuture<Json<T>>;

    fn from_request(req: Request) -> Self::Future {
        match validate(&req) {
            Some(()) => Either::Left(Map::new(
                req.into_body().collect_body(),
                (|e| Ok(Json(serde_json::from_slice(&e?.into_bytes_mut())?))) as JsonMap<Json<T>>,
            )),
            None => Either::Right(ready(Err(JsonFutureError::ContentType))),
        }
    }
}

// ===== IntoResponse =====

impl IntoResponse for Value {
    #[inline]
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        const APPLICATION_JSON: [(HeaderName, HeaderValue); 1] =
            [(CONTENT_TYPE, HeaderValue::from_static("application/json"))];

        match serde_json::to_vec(&self.0) {
            Ok(ok) => (APPLICATION_JSON, ok).into_response(),
            Err(_err) => {
                log!("failed to serialize json response: {_err}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ===== serde =====

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Json<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Json)
    }
}

impl<T: serde::Serialize> serde::Serialize for Json<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

// ===== Error =====

/// Error that can be returned from [`Json`] implementation of [`FromRequest`].
#[derive(Debug)]
pub enum JsonFutureError {
    ContentType,
    Body(BodyError),
    Serde(serde_json::Error),
}

impl From<BodyError> for JsonFutureError {
    fn from(e: BodyError) -> Self {
        Self::Body(e)
    }
}
impl From<serde_json::Error> for JsonFutureError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
    }
}

impl IntoResponse for JsonFutureError {
    fn into_response(self) -> Response {
        match self {
            Self::ContentType => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported media type").into_response()
            }
            Self::Body(error) => error.into_response(),
            Self::Serde(error) => error.into_response(),
        }
    }
}

impl std::error::Error for JsonFutureError {}

impl fmt::Display for JsonFutureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ContentType => f.write_str("unsupported media type"),
            Self::Body(hyper) => hyper.fmt(f),
            Self::Serde(serde) => serde.fmt(f),
        }
    }
}

impl IntoResponse for serde_json::Error {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

