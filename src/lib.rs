//! Web Server and Client Toolkit
#![warn(missing_debug_implementations)]

// NOTE:
// on progress of pulling out modules into other crates
// - serialization
// - service
// take out
// - from request, into response, routing

pub use tcio::ByteStr;

pub mod http;
pub mod headers;
pub mod body;
pub mod request;
pub mod response;
mod ws;

mod task;
pub mod service;
pub mod rt;

