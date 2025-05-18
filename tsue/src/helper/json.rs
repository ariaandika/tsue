use bytes::Bytes;
use http::{HeaderName, HeaderValue, StatusCode, header::CONTENT_TYPE};
use serde::{Serialize, de::DeserializeOwned};
use std::{fmt, marker::PhantomData, pin::Pin, task::{ready, Context, Poll}};

use super::Json;
use crate::{
    request::{FromRequest, Request},
    response::{IntoResponse, Response},
};

// ===== FromRequest =====

type BytesFutureError = <Bytes as FromRequest>::Error;

impl<T: DeserializeOwned> FromRequest for Json<T> {
    type Error = JsonFutureError;
    type Future = JsonFuture<T>;

    fn from_request(req: Request) -> Self::Future {
        let content_type = req.headers().get(CONTENT_TYPE).cloned();
        JsonFuture {
            phase: Phase::P1 { content_type, req: Some(req) },
            _p: PhantomData,
        }
    }
}

pin_project_lite::pin_project! {
    pub struct JsonFuture<T> {
        #[pin]
        phase: Phase<<Bytes as FromRequest>::Future>,
        _p: PhantomData<T>,
    }
}

pin_project_lite::pin_project! {
    #[project = PhaseProj]
    enum Phase<F> {
        P1 { content_type: Option<HeaderValue>, req: Option<Request> },
        P2 { #[pin] f: F }
    }
}

impl<T: DeserializeOwned> Future for JsonFuture<T> {
    type Output = Result<Json<T>, JsonFutureError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project().phase.as_mut().project() {
                PhaseProj::P1 { content_type, req } => {
                    fn validate(ct: Option<&mut HeaderValue>) -> Option<()> {
                        (mime::APPLICATION_JSON == ct?.to_str().ok()?).then_some(())
                    }

                    if validate(content_type.as_mut()).is_none() {
                        return Poll::Ready(Err(JsonFutureError::ContentType))
                    };

                    let f = Bytes::from_request(req.take().unwrap());
                    self.as_mut().project().phase.set(Phase::P2 { f });
                },
                PhaseProj::P2 { f } => {
                    let v = ready!(f.poll(cx)?);
                    let ok = serde_json::from_slice(&v)?;
                    return Poll::Ready(Ok(Json(ok)))
                },
            }
        }
    }
}

// ===== IntoResponse =====

const APPLICATION_JSON: [(HeaderName, HeaderValue); 1] =
    [(CONTENT_TYPE, HeaderValue::from_static("application/json"))];

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(ok) => (APPLICATION_JSON, ok).into_response(),
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
    ContentType,
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
        match self {
            Self::ContentType => (StatusCode::BAD_REQUEST,"invalid content-type").into_response(),
            Self::Hyper(error) => error.into_response(),
            Self::Serde(error) => error.into_response(),
        }
    }
}

impl std::error::Error for JsonFutureError {}

impl fmt::Display for JsonFutureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ContentType => f.write_str("invalid content-type"),
            Self::Hyper(hyper) => hyper.fmt(f),
            Self::Serde(serde) => serde.fmt(f),
        }
    }
}
