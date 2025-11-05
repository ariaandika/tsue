use std::task::Poll;
use tcio::{
    ByteStr,
    bytes::{Buf, BytesMut},
};

use super::{
    error::{HttpError, ErrorKind},
    matches,
};

macro_rules! ready {
    ($e:expr) => {
        match $e {
            Some(ok) => ok,
            None => return Poll::Pending
        }
    };
}

macro_rules! err {
    ($variant:ident) => {
        Poll::Ready(Err(HttpError::from(ErrorKind::$variant)))
    };
}

#[derive(Debug)]
pub struct Header {
    pub name: ByteStr,
    pub value: BytesMut,
}

impl Header {
    #[inline]
    pub fn parse_chunk(bytes: &mut BytesMut) -> Poll<Result<Option<Header>, HttpError>> {
        parse_chunk_header(bytes)
    }
}

fn parse_chunk_header(bytes: &mut BytesMut) -> Poll<Result<Option<Header>, HttpError>> {
    let mut cursor = bytes.cursor_mut();

    match ready!(cursor.next()) {
        b'\r' => match ready!(cursor.next()) {
            b'\n' => {
                cursor.advance_buf();
                return Poll::Ready(Ok(None));
            }
            _ => return err!(InvalidSeparator),
        },
        b'\n' => {
            cursor.advance_buf();
            return Poll::Ready(Ok(None));
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
            return Poll::Pending
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

    Poll::Ready(Ok(Some(Header {
        name,
        value: line,
    })))
}
