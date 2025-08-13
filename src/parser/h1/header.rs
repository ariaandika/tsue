use std::task::{Poll, ready};
use tcio::bytes::{Buf, Bytes, BytesMut, Cursor};

use super::{
    error::{Error, ErrorKind},
    message::find_line_buf,
};

macro_rules! err {
    ($variant:ident) => {
        Poll::Ready(Some(Err(Error::from(ErrorKind::$variant))))
    };
}

#[derive(Debug)]
pub struct Header {
    pub name: BytesMut,
    pub value: BytesMut,
}

pub fn parse_header(bytes: &mut BytesMut) -> Poll<Option<Result<Header, Error>>> {
    let mut line = ready!(find_line_buf(bytes));

    if line.is_empty() {
        return Poll::Ready(None);
    }

    let mut cursor = Cursor::new(&line);

    loop {
        match cursor.next() {
            Some(b':') => break,
            Some(_) => {}
            None => return err!(InvalidSeparator),
        }
    }

    let name = line.split_to(cursor.steps() - 1);
    line.advance(1);

    if !matches!(line.first(), Some(b' ')) {
        return err!(InvalidSeparator);
    }
    line.advance(1);

    Poll::Ready(Some(Ok(Header { name, value: line })))
}
