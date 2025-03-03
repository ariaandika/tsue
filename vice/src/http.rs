//! project specific http protocol
use bytes::Bytes;
use http_body_util::Full;

pub mod from_request;
pub mod into_response;

pub use hyper::body::Incoming as ReqBody;

pub type Request<T = ReqBody> = hyper::http::Request<T>;
pub type Response<T = ResBody> = hyper::http::Response<T>;
pub type ResBody = Full<Bytes>;
