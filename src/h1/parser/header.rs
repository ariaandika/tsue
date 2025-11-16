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

    let mut cursor = bytes.cursor_mut();

    match ready!(cursor.next()) {
        b'\r' => match ready!(cursor.next()) {
            b'\n' => {
                cursor.advance_buf();
                return Result::Ok(None);
            }
            _ => return Result::Err(ParseError::InvalidSeparator),
        },
        b'\n' => {
            cursor.advance_buf();
            return Result::Ok(None);
        }
        _ => {}
    }

    cursor = bytes.cursor_mut();

    let offset = matches::match_header_name! {
        cursor;
        |val,nth| match val {
            b':' => nth,
            _ => return Result::Err(ParseError::InvalidHeader),
        };
        else {
            return ParseResult::Pending
        }
    };

    match ready!(cursor.next()) {
        b' ' => { }
        _ => return Result::Err(ParseError::InvalidSeparator),
    }

    matches::match_header_value!(cursor);

    let crlf = match ready!(cursor.next()) {
        b'\r' => match ready!(cursor.next()) {
            b'\n' => 2,
            _ => return Result::Err(ParseError::InvalidSeparator),
        },
        b'\n' => 1,
        _ => return Result::Err(ParseError::InvalidHeader),
    };

    let mut line = cursor.split_to();
    let name = line.split_to(offset);

    line.advance(b": ".len());
    line.truncate_off(crlf);

    ParseResult::Ok(Some(Header {
        name,
        value: line,
    }))
}
