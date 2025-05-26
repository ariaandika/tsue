use futures_util::FutureExt;
use http::{StatusCode, header::CONTENT_TYPE};
use serde::de::DeserializeOwned;
use std::future::ready;

use super::{Form, macros::derefm};
use crate::{
    body::BodyError,
    request::{FromRequest, Request},
    response::IntoResponse,
};

derefm!(<T>|Form<T>| -> T);

fn validate(req: &Request) -> Option<()> {
    req.headers()
        .get(CONTENT_TYPE)?
        .to_str()
        .ok()?
        .eq_ignore_ascii_case("application/x-www-form-urlencoded")
        .then_some(())
}

// ===== FromRequest =====

type FormMap<V> = fn(Result<crate::body::Collected, BodyError>) -> Result<Form<V>, FormFutureError>;

type FormFuture<V> = futures_util::future::Either<
    futures_util::future::Map<crate::body::Collect, FormMap<V>>,
    std::future::Ready<Result<Form<V>, FormFutureError>>,
>;

impl<T: DeserializeOwned> FromRequest for Form<T> {
    type Error = FormFutureError;
    type Future = FormFuture<T>;

    fn from_request(req: Request) -> Self::Future {
        match validate(&req) {
            Some(()) => req
                .into_body()
                .collect_body()
                .map((|e| Ok(Form(serde_urlencoded::from_bytes(&e?.into_bytes_mut())?))) as FormMap<T>)
                .left_future(),
            None => ready(Err(FormFutureError::ContentType)).right_future(),
        }
    }
}

// ===== serde =====

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Form<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Form)
    }
}

impl<T: serde::Serialize> serde::Serialize for Form<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

// ===== Error =====

#[derive(Debug)]
pub enum FormFutureError {
    ContentType,
    Body(BodyError),
    Serde(serde_urlencoded::de::Error),
}

impl From<BodyError> for FormFutureError {
    fn from(v: BodyError) -> Self {
        Self::Body(v)
    }
}

impl From<serde_urlencoded::de::Error> for FormFutureError {
    fn from(v: serde_urlencoded::de::Error) -> Self {
        Self::Serde(v)
    }
}

impl IntoResponse for FormFutureError {
    fn into_response(self) -> crate::response::Response {
        match self {
            Self::ContentType => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported media type").into_response()
            }
            Self::Body(error) => error.into_response(),
            Self::Serde(error) => error.into_response(),
        }
    }
}

impl std::error::Error for FormFutureError {}

impl std::fmt::Display for FormFutureError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FormFutureError::ContentType => f.write_str("unsupported media type"),
            FormFutureError::Body(error) => error.fmt(f),
            FormFutureError::Serde(error) => error.fmt(f),
        }
    }
}

impl IntoResponse for serde_urlencoded::de::Error {
    fn into_response(self) -> crate::response::Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

