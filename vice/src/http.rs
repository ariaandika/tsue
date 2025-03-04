//! http protocol
use bytes::Bytes;
use http_body_util::Full;

pub mod from_request;
pub mod into_response;

pub use http::Method;
pub use http::StatusCode;
pub use http::request;
pub use http::response;
pub use http::header;
pub use http::status;

pub use hyper::body::Incoming as ReqBody;

#[doc(inline)]
pub use from_request::{FromRequest, FromRequestParts};
#[doc(inline)]
pub use into_response::{IntoResponse, IntoResponseParts};

/// Represents an HTTP request
pub type Request<T = ReqBody> = hyper::http::Request<T>;
/// Represents an HTTP response
pub type Response<T = ResBody> = hyper::http::Response<T>;
/// Represents a response body
pub type ResBody = Full<Bytes>;
