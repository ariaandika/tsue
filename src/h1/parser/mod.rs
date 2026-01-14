//! HTTP/1.1 Parser.
//!
//! [`Reqline`] work on chunk read, given unknown length of bytes, [`Reqline::parse_chunk`] will
//! find the next separator, and advance the bytes to it. If crlf is not found, then it returns
//! [`ParseResult::Pending`].
//!
//! [`Header`] works the same way. Additionally, if [`Header::parse_chunk`] encounter an empty line
//! with crlf, it returns [`ParseResult::Ok(None)`] denoting that its the end of header fields.
//!
//! [`Target`] handles request target with its different representation.
//!
//! [`ParseResult::Pending`]: crate::common::ParseResult::Pending
//! [`ParseResult::Ok(None)`]: crate::common::ParseResult::Ok
mod matches;

mod request;
mod target;
mod header;
mod error;

pub use target::{Target, Kind};
pub use error::ParseError;

pub use request::parse_reqline_chunk;
pub use header::parse_header_chunk;

#[cfg(test)]
mod test;
