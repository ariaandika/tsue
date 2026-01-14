use std::slice::from_raw_parts;
use tcio::bytes::BytesMut;

use super::{Target, error::ParseError, matches};
use crate::common::ParseResult;
use crate::http::{Method, Version};
use crate::proto::Reqline;

const VERSION_SIZE: usize = b"HTTP/1.1".len();

// ===== Request Line =====

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
    let target = Target::new(&method, reqline);

    ParseResult::Ok(Reqline {
        method,
        target,
        version,
    })
}
