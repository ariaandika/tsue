#![deny(clippy::cast_possible_truncation)]
//! HTTP/1.1 Semantics.
//!
//! - [`HttpContext`] per request context
//!
//! - [`BodyDecoder`] message body decoder
//! - [`Coding`] body coding information
//! - [`Chunked`] transfer chunked encoding values
//! - [`Encoding`] chunked encoding kinds
//!
//! - [`HttpContext`] statefull http request context builder
//!
//! # Usage
//!
//! Create [`HttpContext`] after parsing reqest line. Add header for each header parsed. Finally
//! call `build_*` method to retrieve [`HttpContext`], [`BodyDecoder`], and [`Request`].
//!
//! [`Request`]: crate::request::Request
mod chunked;
mod body;

mod context;
mod state;
mod error;

pub(crate) use state::MAX_HEADERS;
use chunked::ChunkedDecoder;

pub use context::HttpContext;
pub use body::{BodyDecoder, Coding, BodyError};
pub use state::{HttpState, write_response};
pub use error::ProtoError;
