//! HTTP/1.1 Parser.
//!
//! [`parse_reqline_chunk`] works on chunked read, given any length of bytes, the parser will find
//! the next separator and advance the bytes to it. If crlf is not found, then the parser returns
//! [`ParseResult::Pending`].
//!
//! [`parse_header_chunk`] works the same way. Additionally, if the parser encounter an empty line
//! with separator, it returns [`ParseResult::Ok(None)`] denoting that its the end of header
//! fields.
//!
//! [`ParseResult::Pending`]: crate::common::ParseResult::Pending
//! [`ParseResult::Ok(None)`]: crate::common::ParseResult::Ok
mod matches;
mod request;
mod header;
mod error;

pub use error::ParseError;
pub use request::parse_reqline_chunk;
pub use header::parse_header_chunk;

#[cfg(test)]
mod test;
