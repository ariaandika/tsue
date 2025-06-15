use bytes::Bytes;
use http::{HeaderName, HeaderValue, header::CONTENT_TYPE};
use hyper::service::Service;
use std::future::{Ready, ready};

use super::{Html, macros::derefm};
use crate::{
    helper::util::{InvalidContentType, validate_content_type},
    request::Request,
    response::{IntoResponse, Response},
};

derefm!(<T>|Html<T>| -> T);

const TEXT_HTML: [(HeaderName, HeaderValue); 1] = [(
    CONTENT_TYPE,
    HeaderValue::from_static("text/html; charset=utf-8"),
)];

impl<T: Clone + Into<Bytes>> Service<Request> for Html<T> {
    type Response = Self;
    type Error = InvalidContentType;
    type Future = Ready<Result<Self, Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        match validate_content_type(&req, "text/html") {
            Some(()) => ready(Ok(self.clone())),
            None => ready(Err(InvalidContentType)),
        }
    }
}

impl<T: Into<Bytes>> IntoResponse for Html<T> {
    fn into_response(self) -> Response {
        (TEXT_HTML, self.0.into()).into_response()
    }
}

impl<T> From<T> for Html<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
