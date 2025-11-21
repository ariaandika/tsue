//! HTTP/1.1 Protocol.
//!
//! - [`parser`] contains HTTP/1.1 parser.
//! - [`spec`] contains HTTP/1.1 semantics.
//! - [`io`] contains IO related APIs
//! - [`connection`] contains the integration of all the components above into single API

pub mod parser;
pub mod spec;
pub mod io;
pub mod io_v2;
pub mod connection;
