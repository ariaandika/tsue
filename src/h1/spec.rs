#![deny(clippy::cast_possible_truncation)]
//! HTTP/1.1 Semantics.
//!
//! - [`HttpContext`] per request context
//!
//! - [`MessageBody`] semantically represent body message
//! - [`Coding`] body coding information
//! - [`Chunked`] transfer chunked encoding values
//! - [`Encoding`] chunked encoding kinds
//!
//! - [`HttpContext`] statefull http request context builder
//!
//! # Usage
//!
//! Create [`HttpContext`] after parsing reqest line. Add header for each header parsed. Finally
//! call `build_*` method to retrieve [`HttpContext`], [`MessageBody`], and [`Request`].
//!
//! [`Request`]: crate::request::Request
mod context;
mod chunked;
mod body;
mod state;
mod error;

pub(crate) use state::MAX_HEADERS;
pub use context::HttpContext;
pub use body::{MessageBody, Coding, BodyError};
pub use state::{HttpState, write_response};
pub use error::ProtoError;
