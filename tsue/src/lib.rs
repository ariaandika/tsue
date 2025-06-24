//! HTTP protocol.

#![warn(missing_debug_implementations)]

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

pub mod http;
mod ws;

pub mod body;

