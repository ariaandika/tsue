use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;


pub type Request<T = Incoming> = hyper::http::Request<T>;
pub type Response<T = ResBody> = hyper::http::Response<T>;
pub type ResBody = Full<Bytes>;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

