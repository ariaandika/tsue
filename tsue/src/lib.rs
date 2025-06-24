#![warn(missing_debug_implementations)]
//! HTTP protocol.
pub use tcio::ByteStr;

// NOTE:
// on progress of pulling out modules into other crates
// what should be kept
// - `http` types
// - request and response
// - websocket
// - serialization
// - service ?
// take out
// - from request, into response, routing
// - service ?

mod common;
pub mod http;
mod ws;

pub mod body;

