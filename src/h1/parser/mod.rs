//! HTTP/1.1 Parser.
//!
//! [`Reqline`] work on matches. Given unknown length of bytes, [`Reqline::parse_chunk`] will find the
//! next separator, and advance the bytes to it. If crlf is not found, then it returns
//! [`Poll::Pending`]
//!
//! [`Header`] works the same way. Additionally, if [`Header::parse_chunk`] encounter an empty line
//! with crlf, it returns [`None`] denoting that its the end of header fields.
//!
//! [`Poll::Pending`]: std::task::Poll::Pending
mod matches;

mod request;
mod target;
mod header;
mod error;

pub use request::Reqline;
pub use target::{Target, Kind};
pub use header::Header;
pub use error::H1ParseError;

#[cfg(test)]
mod test;
