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
//! Create [`HttpState`] after parsing reqest line. Add header for each header parsed. Finally
//! call `build_*` method to retrieve [`HttpContext`], [`BodyDecoder`], and [`Request`].
//!
//! [`Request`]: crate::request::Request

mod state;
mod context;
pub mod error;

pub(crate) use state::{HttpState, insert_header, write_response_head};
pub(crate) use context::HttpContext;
