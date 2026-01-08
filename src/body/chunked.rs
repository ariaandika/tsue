use std::mem;
use std::{num::NonZeroU64, task::Poll};
use tcio::bytes::{Buf, Bytes, BytesMut};

use crate::body::error::BodyError;

const MAX_CHUNKED_SIZE: u64 = u64::MAX >> 1;

#[derive(Clone, Debug)]
pub struct ChunkedCoder {
    /// 0 => Eof,
    /// MAX => Header phase,
    /// _ => Chunk phase,
    raw: u64
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

    fn is_eof(&self) -> bool {
        self.raw == 0
    }

    /// Poll for chunked body, returns `None` if end of chunks found.
    pub(crate) fn decode_chunk(
        &mut self,
        buffer: &mut BytesMut,
    ) -> Poll<Option<Result<BytesMut, BodyError>>> {
        if self.is_eof() {
            return Poll::Ready(None);
        }

        if buffer.is_empty() {
            return Poll::Pending;
        }

        if self.is_header() {
            let Some(digits_len) = buffer.iter().position(|e| !e.is_ascii_hexdigit()) else {
                return Poll::Pending;
            };
            // SAFETY: `is_ascii_hexdigit` is subset of ASCII
            let digits = unsafe { str::from_utf8_unchecked(&buffer[..digits_len]) };
            let Ok(chunk_len) = u64::from_str_radix(digits, 16) else {
                return Poll::Ready(Some(Err(BodyError::InvalidChunked)));
            };
            if chunk_len > MAX_CHUNKED_SIZE {
                return Poll::Ready(Some(Err(BodyError::ChunkTooLarge)));
            }

            // extension / CRLF delimiter
            let trailing_header = match buffer[digits_len] {
                b'\r' => match buffer.get(digits_len + 1) {
                    Some(b'\n') => 2,
                    Some(_) => return Poll::Ready(Some(Err(BodyError::InvalidChunked))),
                    None => return Poll::Pending,
                },
                b';' => match buffer[digits_len..].iter().position(|&e| e == b'\n') {
                    // trailing is index of '\n', therefore `+ 1` to include the '\n'
                    Some(trailing) => trailing + 1,
                    None => return Poll::Pending,
                },
                _ => return Poll::Ready(Some(Err(BodyError::InvalidChunked))),
            };

            self.raw = chunk_len;

            if chunk_len == 0 {
                match buffer[digits_len..].first_chunk::<2>() {
                    Some(b"\r\n") => buffer.advance(2),
                    Some(_) => return Poll::Ready(Some(Err(BodyError::InvalidChunked))),
                    None => return Poll::Pending,
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
                return Poll::Pending;
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
                    let crlf = unsafe { offset_chunk!{buffer, rem, 2} };
                    if crlf != b"\r\n" {
                        return Poll::Ready(Some(Err(BodyError::InvalidChunked)));
                    }

                    self.set_header_phase();

                    let b = buffer.split_to(rem);
                    buffer.advance(2);
                    b
                },
            };

            Poll::Ready(Some(Ok(chunk)))
        }
    }

    pub fn encode_chunk<B: Buf>(
        &mut self,
        chunk: B,
        write_buffer: &mut BytesMut,
        is_last_chunk: bool,
    ) -> EncodedBuf<B> {
        let Some(clen) = NonZeroU64::new(chunk.remaining() as u64) else {
            // caller give empty chunk
            return EncodedBuf::exact(chunk);
        };

        const CRLF: [u8; 2] = *b"\r\n";
        const CRLF_LEN: usize = CRLF.len();

        write_buffer.reserve(<usize as itoa::Integer>::MAX_STR_LEN + CRLF_LEN);
        let header: &mut [u8] = unsafe { mem::transmute(write_buffer.spare_capacity_mut()) };

        let mut b = itoa::Buffer::new();
        let s = b.format(clen.get()).as_bytes();
        let len = s.len();

        unsafe {
            std::ptr::copy_nonoverlapping(s.as_ptr(), header.as_mut_ptr(), len);
            std::ptr::copy_nonoverlapping(CRLF.as_ptr(), header.as_mut_ptr().add(len), 2);
        }
        let header = write_buffer.split_to(len + CRLF_LEN).freeze();

        let crlf = (is_last_chunk as usize) << 1;

        EncodedBuf::chunks(header, chunk, &CRLF[..crlf])
    }
}

/// The return type for encoded message body chunk.
///
/// The returned bytes must be written in following order: `header`, `chunk`, then `trail`.
#[derive(Debug)]
pub struct EncodedBuf<B> {
    pub header: Bytes,
    pub chunk: B,
    pub trail: &'static [u8],
}

impl<B> EncodedBuf<B> {
    pub fn exact(chunk: B) -> Self {
        Self { header: Bytes::new(), chunk, trail: b"" }
    }

    pub fn chunks(header: Bytes, chunk: B, trail: &'static [u8]) -> Self {
        Self { header, chunk, trail }
    }
}


/// unchecked `&bytes[offset..offset + len]`
macro_rules! offset_chunk {
    ($b:ident, $offset:expr, $len:expr) => {
        debug_assert!($b.len() >= $offset + $len);
        &*($b.as_ptr().add($offset).cast::<[u8; $len]>())
    };
}

use {offset_chunk};
