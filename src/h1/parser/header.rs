use tcio::bytes::{Buf, BytesMut};

use super::{error::ParseError, matches};
use crate::common::ParseResult;

macro_rules! ready {
    ($e:expr) => {
        match $e {
            Some(ok) => ok,
            None => return ParseResult::Pending
        }
    };
}

#[derive(Debug)]
pub struct Header {
    pub name: BytesMut,
    pub value: BytesMut,
}

impl Header {
    #[inline]
    pub fn parse_chunk(bytes: &mut BytesMut) -> ParseResult<Option<Header>, ParseError> {
        parse_chunk_header(bytes)
    }
}

fn parse_chunk_header(bytes: &mut BytesMut) -> ParseResult<Option<Header>, ParseError> {
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
