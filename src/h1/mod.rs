//! HTTP/1.1 Protocol.
//!
//! - [`parser`] contains HTTP/1.1 parser.
//! - [`proto`] contains HTTP/1.1 semantics.
//! - [`io`] contains IO related APIs
//! - [`driver`] contains the integration of all the components above into single API

pub mod parser;
pub mod proto;
pub mod io;
pub mod driver;
