use std::task::Poll;
use tcio::bytes::{BytesMut, Cursor};

const CHUNK_SIZE: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([128; CHUNK_SIZE]);
const LSB: usize = usize::from_ne_bytes([1; CHUNK_SIZE]);

pub fn find_line_buf(bytes: &mut BytesMut) -> Poll<BytesMut> {
    let mut buf = bytes.as_slice();
    match find_line(&mut buf) {
        Poll::Ready(line) => {
            let line_len = line.len();
            let offset = buf.as_ptr().addr() - bytes.as_ptr().addr();
            let mut line_buf = bytes.split_to(offset);
            line_buf.truncate_off(offset - line_len);
            Poll::Ready(line_buf)
        },
        Poll::Pending => Poll::Pending,
    }
}

/// Find line ends with CRLF or LF.
pub const fn find_line<'a>(bytes: &mut &'a [u8]) -> Poll<&'a [u8]> {
    const LF: usize = usize::from_ne_bytes([b'\n'; CHUNK_SIZE]);

    let mut cursor = Cursor::new(bytes);

    while let Some(chunk) = cursor.peek_chunk::<CHUNK_SIZE>() {
        let value = usize::from_ne_bytes(*chunk);
        let lf_xor = value ^ LF;
        let lf_result = lf_xor.wrapping_sub(LSB) & !lf_xor & MSB;

        if lf_result != 0 {
            let lf_pos = (lf_result.trailing_zeros() / 8) as usize;

            cursor.advance(lf_pos + 1/* = eat the '\n' in buffer */);
            *bytes = cursor.as_slice();

            match cursor.peek_prev_chunk() {
                Some(b"\r\n") => cursor.step_back(2),
                _ => cursor.step_back(1),
            }

            return Poll::Ready(cursor.advanced_slice());
        }

        cursor.advance(CHUNK_SIZE);
    }

    while let Some(b) = cursor.next() {
        if b == b'\n' {
            *bytes = cursor.as_slice();

            match cursor.peek_prev_chunk() {
                Some(b"\r\n") => cursor.step_back(2),
                _ => cursor.step_back(1),
            }

            return Poll::Ready(cursor.advanced_slice());
        }
    }

    Poll::Pending
}
