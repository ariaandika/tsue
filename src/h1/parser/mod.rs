//! HTTP/1.1 Parser.
//!
//! [`parse_reqline_chunk`] works on chunked bytes, given any length of bytes, the parser will find
//! the next separator and advance the bytes to it. If crlf is not found, then the parser returns
//! [`ParseResult::Pending`], where more bytes is required to complete parsing.
//!
//! [`parse_header_chunk`] works the same way. Additionally, if the parser encounter an empty line
//! with separator, it returns [`ParseResult::Ok(None)`] denoting that its the end of header
//! fields.
//!
//! [`ParseResult::Pending`]: crate::common::ParseResult::Pending
//! [`ParseResult::Ok(None)`]: crate::common::ParseResult::Ok
mod matches;

macro_rules! ready {
    ($e:expr) => {
        match $e {
            Some(ok) => ok,
            None => return ParseResult::Pending
        }
    };
}

use std::slice::from_raw_parts;
use tcio::bytes::{Buf, BytesMut};

use crate::common::ParseResult;
use crate::http::{Method, Version};
use crate::proto::error::ParseError;
use crate::proto::{Header, Reqline};

#[cfg(test)]
mod test;

const VERSION_SIZE: usize = b"HTTP/1.1".len();

// ===== Request Line =====

/// Parser request control data.
///
/// This function performs a chunked parsing, see [module level documentation] for more details.
///
/// [module level documentation]: crate::h1::parser
pub fn parse_reqline_chunk(bytes: &mut BytesMut) -> ParseResult<Reqline, ParseError> {
    use ParseResult as Result;

    if bytes.is_empty() {
        return Result::Pending;
    }

    let mut reqline = {
        let mut state = bytes.as_slice();

        let delim = matches::split_crlf!(state else {
            return ParseResult::Pending
        });

        let crlf = match delim {
            b'\r' => match state.split_first() {
                Some((b'\n', rest)) => {
                    state = rest;
                    2
                },
                Some(_) => return Result::Err(ParseError::InvalidSeparator),
                None => return Result::Pending,
            },
            b'\n' => 1,
            _ => return Result::Err(ParseError::InvalidSeparator),
        };

        let mut reqline = bytes.split_to_ptr(state.as_ptr());
        reqline.truncate_off(crlf);
        reqline
    };

    let mut target = reqline.as_slice();

    let method = {
        let mut state = target;

        while let [byte, rest @ ..] = state {
            if !matches::is_method(*byte) {
                if *byte == b' ' {
                    target = rest;
                    break;
                } else {
                    return Result::Err(ParseError::InvalidMethod);
                }
            }
            state = rest;
        }

        let method = unsafe {
            let start = reqline.as_ptr();
            let len = state.as_ptr().offset_from_unsigned(start);
            from_raw_parts(start, len)
        };

        match Method::from_bytes(method) {
            Some(ok) => {
                ok
            },
            _ => return Result::Err(ParseError::UnknownMethod),
        }
    };

    let version = {
        let Some(([rest @ .., b' '], version)) = target.split_last_chunk::<VERSION_SIZE>() else {
            return Result::Err(ParseError::InvalidSeparator);
        };

        target = rest;

        match Version::from_bytes(version) {
            Some(ok) => ok,
            None => return Result::Err(ParseError::UnsupportedVersion),
        }
    };

    // SAFETY: `target` is only sliced within `target` bounds itself
    unsafe {
        let len = target.len();
        reqline.advance_to_ptr(target.as_ptr());
        reqline.truncate(len);
    }

    ParseResult::Ok(Reqline {
        method,
        target: reqline,
        version,
    })
}

/// Parser header field.
///
/// Returns `ParseResult::Ok(None)` when encounter an empty line with separator.
///
/// This function performs a chunked parsing, see [module level documentation] for more details.
///
/// [module level documentation]: crate::h1::parser
pub fn parse_header_chunk(bytes: &mut BytesMut) -> ParseResult<Option<Header>, ParseError> {
    use ParseResult as Result;

    if let b @ (b'\r' | b'\n') = ready!(bytes.first()) {
        let adv = match (b, bytes.get(1)) {
            (b'\r', Some(b'\n')) => 2,
            (b'\r', None) => return Result::Err(ParseError::InvalidSeparator),
            (b'\n', _) => 1,
            _ => unreachable!(),
        };
        bytes.advance(adv);
        return Result::Ok(None);
    }

    let mut line = {
        let mut state = bytes.as_slice();
        let delim = matches::split_crlf!(state else {
            return Result::Pending
        });

        let crlf = match delim {
            b'\r' => match state.split_first() {
                Some((b'\n', rest)) => {
                    state = rest;
                    2
                },
                Some(_) => return Result::Err(ParseError::InvalidSeparator),
                None => return Result::Pending,
            },
            b'\n' => 1,
            _ => return Result::Err(ParseError::InvalidSeparator),
        };
        let mut line = bytes.split_to_ptr(state.as_ptr());
        line.truncate_off(crlf);
        line
    };

    let mut state = line.as_slice();
    let delim = matches::split_header_name!(state else {
        return Result::Err(ParseError::InvalidHeader)
    });
    if delim != b':' {
        return Result::Err(ParseError::InvalidHeader)
    }

    let mut delim_len = 1;
    while let Some((b' ', rest)) = state.split_first() {
        delim_len += 1;
        state = rest;
    }

    let mut name = line.split_to_ptr(state.as_ptr());
    name.truncate_off(delim_len);

    ParseResult::Ok(Some(Header {
        name,
        value: line,
    }))
}
