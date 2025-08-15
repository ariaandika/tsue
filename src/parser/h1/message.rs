use std::task::{ready, Poll};
use tcio::bytes::{Buf, BytesMut, Cursor};

use crate::parser::simd::match_swar;

pub fn find_line_buf(bytes: &mut BytesMut) -> Poll<BytesMut> {
    let mut cursor = Cursor::new(bytes.as_slice());

    ready!(match_swar!(cursor, =b'\n'));

    let crlf = match cursor.peek_prev_chunk() {
        Some(b"\r\n") => 2,
        _ => 1,
    };
    cursor.step_back(crlf);

    let line = bytes.split_to(cursor.steps());
    bytes.advance(crlf);

    Poll::Ready(line)
}

