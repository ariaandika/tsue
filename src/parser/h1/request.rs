use std::task::{Poll, ready};
use tcio::bytes::{Buf, BytesMut, Cursor};

use super::{
    error::{Error, ErrorKind},
    message::find_line_buf,
};
use crate::http::{Method, Version};

macro_rules! err {
    ($variant:ident) => {
        Poll::Ready(Err(Error::from(ErrorKind::$variant)))
    };
}

const VERSION_SIZE: usize = b"HTTP/1.1".len();

#[derive(Debug)]
pub struct Reqline {
    pub method: Method,
    pub target: BytesMut,
    pub version: Version,
}

pub fn parse_reqline(bytes: &mut BytesMut) -> Poll<Result<Reqline, Error>> {
    let mut reqline = ready!(find_line_buf(bytes));

    let method = {
        let mut cursor = Cursor::new(&reqline);

        loop {
            match cursor.next() {
                Some(b' ') => break,
                Some(_) => {}
                None => return err!(TooLong),
            }
        }

        cursor.step_back(1);

        let Some(ok) = Method::from_bytes(cursor.advanced_slice()) else {
            return err!(UnknownMethod);
        };

        reqline.advance(cursor.steps() + 1);
        ok
    };

    let version = {
        let Some((rest, version)) = reqline.split_last_chunk::<VERSION_SIZE>() else {
            return err!(TooShort);
        };

        let Some(ok) = Version::from_bytes(version) else {
            return err!(UnsupportedVersion);
        };

        if !matches!(rest.last(), Some(b' ')) {
            return err!(InvalidSeparator);
        }

        reqline.truncate(reqline.len() - (VERSION_SIZE + 1));
        ok
    };

    Poll::Ready(Ok(Reqline {
        method,
        target: reqline,
        version,
    }))
}
