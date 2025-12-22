//! HTTP/1.1 Protocol.
//!
//! - [`parser`] contains HTTP/1.1 parser.
//! - [`connection`] contains the integration of all the components above into single API

pub mod parser;
pub mod connection;
