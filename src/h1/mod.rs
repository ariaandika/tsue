//! HTTP/1.1 Protocol.
//!
//! - [`parser`] contains HTTP/1.1 protocol parser.
//! - [`proto`] contains HTTP/1.1 related logic.
//! - [`io`] contains IO related APIs
//! - [`driver`] contains the integration of all the components above into single API

/// Usage
///
/// [`parser`], parse http request line, request target, and headers
///
/// [`io`], encoding aware io reading and streaming
///
/// [`proto`], semantically build request object
mod doc {}

pub mod parser;
pub mod io;

pub mod proto;
pub mod driver;

pub mod error;
