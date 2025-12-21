//! HTTP/1.1 Protocol.
//!
//! - [`parser`] contains HTTP/1.1 parser.
//! - [`spec`] contains HTTP/1.1 semantics.
//! - [`connection`] contains the integration of all the components above into single API

mod shared;

pub mod parser;
pub mod spec;
pub mod connection;
