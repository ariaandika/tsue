use std::task::Poll;
use tcio::bytes::{Buf, BytesMut};

use super::{
    error::{Error, ErrorKind},
    simd,
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
        Poll::Ready(Err(Error::from(ErrorKind::$variant)))
    };
}

#[derive(Debug)]
pub struct Header {
    pub name: BytesMut,
    pub value: BytesMut,
}

impl Header {
    #[inline]
    pub fn matches(bytes: &mut BytesMut) -> Poll<Result<Option<Header>, Error>> {
        matches_header(bytes)
    }
}

fn matches_header(bytes: &mut BytesMut) -> Poll<Result<Option<Header>, Error>> {
    let mut cursor = bytes.cursor_mut();

    match ready!(cursor.next()) {
        b'\r' => match ready!(cursor.next()) {
            b'\n' => {
                bytes.advance(2);
                return Poll::Ready(Ok(None));
            }
            _ => return err!(InvalidSeparator),
        },
        b'\n' => {
            bytes.advance(1);
            return Poll::Ready(Ok(None));
        }
        _ => {}
    }

    cursor = bytes.cursor_mut();

    let offset = simd::match_header_name! {
        cursor;
        |val,nth| match val {
            b':' => nth,
            _ => return err!(InvalidChar),
        };
        else {
            return Poll::Pending
        }
    };

    match ready!(cursor.next()) {
        b' ' => { }
        _ => return err!(InvalidSeparator),
    }

    simd::match_crlf!(cursor);

    let crlf = match ready!(cursor.next()) {
        b'\r' => match ready!(cursor.next()) {
            b'\n' => 2,
            _ => return err!(InvalidSeparator),
        },
        b'\n' => 1,
        _ => return err!(InvalidChar),
    };

    let mut line = cursor.split_to();

    let name = line.split_to(offset);

    line.advance(b": ".len());
    line.truncate_off(crlf);

    Poll::Ready(Ok(Some(Header {
        name,
        value: line,
    })))
}
