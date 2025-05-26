use bytes::Bytes;
use http::{HeaderValue, StatusCode, header::CONTENT_TYPE};
use serde::de::DeserializeOwned;
use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::{Form, macros::derefm};
use crate::{
    body::BodyError,
    request::{FromRequest, Request},
    response::IntoResponse,
};

derefm!(<T>|Form<T>| -> T);

// ===== FromRequest =====

impl<T: DeserializeOwned> FromRequest for Form<T> {
    type Error = FormFutureError;

    type Future = FormFuture<T>;

    fn from_request(req: Request) -> Self::Future {
        let content_type = req.headers().get(CONTENT_TYPE);

        fn validate(ct: Option<&HeaderValue>) -> Option<()> {
            ct?.to_str().ok()?.eq_ignore_ascii_case("application/x-www-form-urlencoded").then_some(())
        }

        if validate(content_type).is_some() {
            FormFuture::Ok {
                ok: Bytes::from_request(req),
            }
        } else {
            FormFuture::Err {
                err: Some(FormFutureError::ContentType),
                _p: PhantomData,
            }
        }
    }
}

pin_project_lite::pin_project! {
    #[project = FormProj]
    pub enum FormFuture<T> {
        Ok { #[pin] ok: <Bytes as FromRequest>::Future },
        Err { err: Option<FormFutureError>, _p: PhantomData<T> },
    }
}

impl<T: DeserializeOwned> Future for FormFuture<T> {
    type Output = Result<Form<T>, FormFutureError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ok = match self.project() {
            FormProj::Ok { ok } => ready!(ok.poll(cx)?),
            FormProj::Err { err, .. } => return Poll::Ready(Err(err.take().unwrap())),
        };
        let ok = serde_urlencoded::from_bytes(&ok)?;
        Poll::Ready(Ok(Form(ok)))
    }
}

// ===== Error =====

#[derive(Debug)]
pub enum FormFutureError {
    ContentType,
    Serde(serde_urlencoded::de::Error),
    Body(BodyError),
}

impl From<serde_urlencoded::de::Error> for FormFutureError {
    fn from(v: serde_urlencoded::de::Error) -> Self {
        Self::Serde(v)
    }
}

impl From<BodyError> for FormFutureError {
    fn from(v: BodyError) -> Self {
        Self::Body(v)
    }
}

impl std::error::Error for FormFutureError {}

impl std::fmt::Display for FormFutureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormFutureError::ContentType => f.write_str("invalid media type"),
            FormFutureError::Serde(error) => error.fmt(f),
            FormFutureError::Body(error) => error.fmt(f),
        }
    }
}

impl IntoResponse for FormFutureError {
    fn into_response(self) -> crate::response::Response {
        match self {
            Self::ContentType => (StatusCode::BAD_REQUEST,"invalid content-type").into_response(),
            Self::Body(error) => error.into_response(),
            Self::Serde(error) => error.into_response(),
        }
    }
}

impl IntoResponse for serde_urlencoded::de::Error {
    fn into_response(self) -> crate::response::Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}
