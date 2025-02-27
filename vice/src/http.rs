use std::convert::Infallible;

use crate::body::{Body, ResBody};

pub type Request<B = Body> = http::Request<B>;
pub type Response<B = ResBody> = http::Response<B>;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        <_>::default()
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::new(ResBody::Bytes(self.into_bytes().into()))
    }
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Response {
        <_>::default()
    }
}


