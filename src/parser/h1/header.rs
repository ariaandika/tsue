use std::task::Poll;
use tcio::bytes::{Buf, BytesMut};

use super::{
    error::{Error, ErrorKind},
    simd,
};

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

    match cursor.next() {
        Some(b'\n') => {
            bytes.advance(1);
            return Poll::Ready(Ok(None));
        }
        Some(b'\r') => match cursor.next() {
            Some(b'\n') => {
                bytes.advance(2);
                return Poll::Ready(Ok(None));
            }
            Some(_) => return err!(InvalidSeparator),
            None => return Poll::Pending,
        },
        Some(_) => {}
        None => return Poll::Pending,
    }

    cursor = bytes.cursor_mut();

    simd::match_crlf!(cursor);

    let crlf = match cursor.next().unwrap() {
        b'\n' => 1,
        b'\r' => match cursor.next() {
            Some(b'\n') => 2,
            Some(_) => return err!(InvalidSeparator),
            None => return Poll::Pending,
        },
        _ => return err!(InvalidChar),
    };

    let mut header_line = cursor.split_to();
    header_line.truncate_off(crlf);

    let name = {
        let mut cursor = header_line.cursor_mut();

        loop {
            match cursor.next() {
                Some(b':') => break,
                Some(_) => {}
                None => return err!(InvalidSeparator),
            }
        }
        cursor.step_back(1);

        let name = cursor.split_to();
        let Some(b": ") = cursor.next_chunk() else {
            return err!(InvalidSeparator);
        };
        cursor.advance_buf();
        name
    };

    Poll::Ready(Ok(Some(Header {
        name,
        value: header_line,
    })))
}
