use tcio::{
    ByteStr,
    bytes::{Buf, BytesMut},
};

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

macro_rules! err {
    ($variant:ident) => {
        ParseResult::Err(ParseError::from(super::error::ParseErrorKind::$variant))
    };
}

#[derive(Debug)]
pub struct Header {
    pub name: ByteStr,
    pub value: BytesMut,
}

impl Header {
    #[inline]
    pub fn parse_chunk(bytes: &mut BytesMut) -> ParseResult<Option<Header>, ParseError> {
        parse_chunk_header(bytes)
    }
}

fn parse_chunk_header(bytes: &mut BytesMut) -> ParseResult<Option<Header>, ParseError> {
    let mut cursor = bytes.cursor_mut();

    match ready!(cursor.next()) {
        b'\r' => match ready!(cursor.next()) {
            b'\n' => {
                cursor.advance_buf();
                return ParseResult::Ok(None);
            }
            _ => return err!(InvalidSeparator),
        },
        b'\n' => {
            cursor.advance_buf();
            return ParseResult::Ok(None);
        }
        _ => {}
    }

    cursor = bytes.cursor_mut();

    let offset = matches::match_header_name! {
        cursor;
        |val,nth| match val {
            b':' => nth,
            _ => return err!(InvalidHeader),
        };
        else {
            return ParseResult::Pending
        }
    };

    match ready!(cursor.next()) {
        b' ' => { }
        _ => return err!(InvalidSeparator),
    }

    matches::match_header_value!(cursor);

    let crlf = match ready!(cursor.next()) {
        b'\r' => match ready!(cursor.next()) {
            b'\n' => 2,
            _ => return err!(InvalidSeparator),
        },
        b'\n' => 1,
        _ => return err!(InvalidHeader),
    };

    let mut line = cursor.split_to();

    // SAFETY: `match_header_name!` checks for valid ASCII
    let name = unsafe { ByteStr::from_utf8_unchecked(line.split_to(offset).freeze()) };

    line.advance(b": ".len());
    line.truncate_off(crlf);

    ParseResult::Ok(Some(Header {
        name,
        value: line,
    }))
}
