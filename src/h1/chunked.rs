use std::task::Poll::{self, *};
use tcio::bytes::{Buf, BytesMut};
use tcio::num::itoa;

use crate::body::error::BodyError;

use BodyError as E;

const MAX_CHUNKED_SIZE: u64 = u64::MAX >> 1;

#[derive(Clone, Debug)]
pub(crate) struct ChunkedCoder {
    /// 0 => Eof,
    /// MAX => Header phase,
    /// _ => Chunk phase,
    raw: u64
}

/// The return type for encoded message body chunk.
///
/// The returned bytes must be written in following order: `header`, `chunk`, then `trail`.
#[derive(Debug)]
pub(crate) struct EncodedChunk<B> {
    pub data: B,
    pub trail: &'static [u8],
}

impl ChunkedCoder {
    pub(crate) fn new() -> Self {
        Self { raw: u64::MAX }
    }

    fn set_header_phase(&mut self) {
        self.raw = u64::MAX
    }

    fn is_header(&self) -> bool {
        self.raw == u64::MAX
    }

    pub fn is_eof(&self) -> bool {
        self.raw == 0
    }

    /// Poll for chunked body, returns `None` if end of chunks found.
    pub(crate) fn decode_chunk(
        &mut self,
        buffer: &mut BytesMut,
    ) -> Poll<Option<Result<BytesMut, BodyError>>> {
        if self.is_eof() {
            return Ready(None);
        }

        if buffer.is_empty() {
            return Pending;
        }

        if self.is_header() {
            let Some(digits_len) = buffer.iter().position(|e| !e.is_ascii_hexdigit()) else {
                return Pending;
            };
            // SAFETY: `is_ascii_hexdigit` is subset of ASCII
            let digits = unsafe { str::from_utf8_unchecked(&buffer[..digits_len]) };
            let Ok(chunk_len) = u64::from_str_radix(digits, 16) else {
                return Ready(Some(Err(E::InvalidChunked)));
            };
            if chunk_len > MAX_CHUNKED_SIZE {
                return Ready(Some(Err(E::ChunkTooLarge)));
            }

            // extension / CRLF delimiter
            let trailing_header = match buffer[digits_len] {
                b'\r' => match buffer.get(digits_len + 1) {
                    Some(b'\n') => 2,
                    Some(_) => return Ready(Some(Err(E::InvalidChunked))),
                    None => return Pending,
                },
                b';' => match buffer[digits_len..].iter().position(|&e| e == b'\n') {
                    // trailing is index of '\n', therefore `+ 1` to include the '\n'
                    Some(trailing) => trailing + 1,
                    None => return Pending,
                },
                _ => return Ready(Some(Err(E::InvalidChunked))),
            };

            self.raw = chunk_len;

            if chunk_len == 0 {
                match buffer[digits_len..].first_chunk::<2>() {
                    Some(b"\r\n") => buffer.advance(2),
                    Some(_) => return Ready(Some(Err(E::InvalidChunked))),
                    None => return Pending,
                }
            }

            buffer.advance(digits_len + trailing_header);
            self.decode_chunk(buffer)

            // ...
        } else {
            let len = buffer.len() as u64;
            let remaining = self.raw;

            if matches!(len.wrapping_sub(remaining), 0 | u64::MAX)  {
                // if the buffer is more than remaining, it needs to at least have the CRLF
                return Pending;
            }

            let chunk = match remaining.checked_sub(len) {
                // buffer contains less than or equal to the remaining chunk
                Some(leftover) => {
                    debug_assert!(leftover > 0);
                    self.raw = leftover;
                    buffer.split()
                },
                // buffer contains larger than the remaining chunk
                None => {
                    let rem = remaining as usize;

                    // SAFETY: checked that buffer remainder left is at least 2 bytes
                    let crlf = unsafe { &*buffer.as_ptr().add(rem).cast::<[u8; 2]>() };
                    if crlf != b"\r\n" {
                        return Ready(Some(Err(E::InvalidChunked)));
                    }

                    self.set_header_phase();

                    let b = buffer.split_to(rem);
                    buffer.advance(2);
                    b
                },
            };

            Ready(Some(Ok(chunk)))
        }
    }

    pub fn encode_chunk<B: Buf>(
        &mut self,
        data: B,
        is_last_chunk: bool,
        write_buffer: &mut BytesMut,
    ) -> EncodedChunk<B> {
        debug_assert!(data.has_remaining());

        const TRAILING: &[u8; 7] = b"\r\n0\r\n\r\n";

        write_buffer.extend_from_slice(itoa().format(data.remaining() as u64).as_bytes());
        write_buffer.extend_from_slice(b"\r\n");

        // if is_last_chunk { 7 } else { 2 }
        let trail = 2 + (is_last_chunk as usize * 5);

        EncodedChunk { data, trail: &TRAILING[..trail] }
    }
}
