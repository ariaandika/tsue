use std::task::{ready, Poll};
use tcio::bytes::{Buf, BytesMut, Cursor};

pub fn find_line_buf(bytes: &mut BytesMut) -> Poll<BytesMut> {
    let mut cursor = Cursor::new(bytes.as_slice());
    ready!(find_line2(&mut cursor));
    let crlf = match cursor.peek_prev_chunk() {
        Some(b"\r\n") => 2,
        _ => 1,
    };
    cursor.step_back(crlf);
    let line = bytes.split_to(cursor.steps());
    bytes.advance(crlf);
    Poll::Ready(line)
}

/// Find line ends with CRLF or LF.
const fn find_line2(cursor: &mut Cursor) -> Poll<()> {
    const CHUNK_SIZE: usize = size_of::<usize>();
    const MSB: usize = usize::from_ne_bytes([128; CHUNK_SIZE]);
    const LSB: usize = usize::from_ne_bytes([1; CHUNK_SIZE]);
    const LF: usize = usize::from_ne_bytes([b'\n'; CHUNK_SIZE]);

    while let Some(chunk) = cursor.peek_chunk::<CHUNK_SIZE>() {
        let value = usize::from_ne_bytes(*chunk);
        let lf_xor = value ^ LF;
        let lf_result = lf_xor.wrapping_sub(LSB) & !lf_xor & MSB;

        if lf_result != 0 {
            let lf_pos = (lf_result.trailing_zeros() / 8) as usize;
            cursor.advance(lf_pos + 1);
            return Poll::Ready(());
        }

        cursor.advance(CHUNK_SIZE);
    }

    while let Some(b) = cursor.next() {
        if b == b'\n' {
            return Poll::Ready(());
        }
    }

    Poll::Pending
}
