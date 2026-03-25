use std::task::Poll::{self, *};
use tcio::bytes::{Buf, BytesMut};

use crate::body::error::BodyError;

use BodyError as E;

/// 1 MB
const MAX_CHUNKED_SIZE: u32 = 1000 * 1024;

/// Chunked transfer decoding and encoding.
#[derive(Clone, Debug)]
pub(crate) struct ChunkedCoder {
    /// 0 => Eof,
    /// MAX => Header phase,
    /// _ => Chunk phase,
    raw: u32
}

/// Encoded chunk transfer data.
///
/// Use [`Buf`] implementation for io write.
#[derive(Debug)]
pub(crate) struct EncodedChunk<B> {
    data: B,
    suffix: &'static [u8],
}

impl<B: Buf> Buf for EncodedChunk<B> {
    fn remaining(&self) -> usize {
        self.data.remaining() + self.suffix.len()
    }

    fn chunk(&self) -> &[u8] {
        if self.data.has_remaining() {
            self.data.chunk()
        } else {
            self.suffix
        }
    }

    fn advance(&mut self, cnt: usize) {
        let data_rem = self.data.remaining();
        self.data.advance(data_rem.min(cnt));
        if let Some(rem) = cnt.checked_sub(data_rem) {
            self.suffix = &self.suffix[rem..];
        }
    }

    fn chunks_vectored<'a>(&'a self, dst: &mut [std::io::IoSlice<'a>]) -> usize {
        let cnt = self.data.chunks_vectored(dst);
        if let Some(io_mut) = dst.get_mut(cnt) {
            *io_mut = std::io::IoSlice::new(self.suffix);
            cnt + 1
        } else {
            cnt
        }
    }
}

impl ChunkedCoder {
    pub(crate) fn new() -> Self {
        Self { raw: u32::MAX }
    }

    fn set_header_phase(&mut self) {
        self.raw = u32::MAX
    }

    fn is_header(&self) -> bool {
        self.raw == u32::MAX
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
            const EOF: &[u8; 5] = b"0\r\n\r\n";
            if buffer.first_chunk() == Some(EOF) {
                buffer.advance(EOF.len());
                self.raw = 0;
                return Ready(None);
            }

            // chunk-size
            let Some(digits_len) = buffer.iter().position(|e| !e.is_ascii_hexdigit()) else {
                return Pending;
            };
            // SAFETY: `is_ascii_hexdigit` is subset of ASCII
            let digits = unsafe { str::from_utf8_unchecked(&buffer[..digits_len]) };
            let Ok(chunk_len) = u32::from_str_radix(digits, 16) else {
                return Ready(Some(Err(E::InvalidChunked)));
            };
            if chunk_len > MAX_CHUNKED_SIZE {
                return Ready(Some(Err(E::ChunkTooLarge)));
            }

            // suffix
            let Some(suffix) = buffer.get(digits_len..digits_len + 1) else {
                return Pending;
            };
            let suffix_len = if suffix == b"\r\n" {
                // CRLF
                2
            } else {
                // chunk-ext
                // currently, extensions is ignored
                let Some(line) = crate::matches::find_byte::<b'\n'>(&buffer[digits_len..]) else {
                    return Pending
                };
                let Some(&suffix) = line.last() else {
                    return Ready(Some(Err(E::InvalidChunked)));
                };
                if suffix != b'\r' {
                    return Ready(Some(Err(E::InvalidChunked)));
                }
                line.len() + 1
            };

            self.raw = chunk_len;
            buffer.advance(digits_len + suffix_len);

            if chunk_len == 0 {
                return Ready(None);
            }
        }

        // `MAX_CHUNKED_SIZE` guarantee this will not truncate the chunk
        let read = buffer.len() as u32;
        let remaining = self.raw;

        match remaining.checked_sub(read) {
            // buffer contains less than the remaining chunk
            Some(leftover) => {
                // make sure the damn CRLF is also read
                if leftover == 0 {
                    return Pending;
                }
                self.raw = leftover;
                Ready(Some(Ok(buffer.split())))
            },
            // buffer contains more than the remaining chunk
            None => {
                let rem = remaining as usize;

                // make sure the damn CRLF is also read
                let Some(mut bytes) = buffer.try_split_to(rem + 2) else {
                    return Pending;
                };

                let rem = remaining as usize;

                // SAFETY: `rem + 2 >= 2`
                let crlf = unsafe { &*buffer.as_ptr().add(rem).cast::<[u8; 2]>() };
                let b"\r\n" = crlf else {
                    return Ready(Some(Err(E::InvalidChunked)));
                };
                bytes.truncate(rem);

                self.set_header_phase();
                Ready(Some(Ok(bytes)))
            },
        }
    }

    pub fn encode_chunk<B: Buf>(
        &mut self,
        data: B,
        is_last_chunk: bool,
        write_buffer: &mut BytesMut,
    ) -> EncodedChunk<B> {
        debug_assert!(data.has_remaining());

        const SUFFIX: &[u8; 7] = b"\r\n0\r\n\r\n";

        // TODO: use non-fmt integer to hex
        use std::io::Write;
        let _ = write!(write_buffer, "{:x}", data.remaining());

        write_buffer.extend_from_slice(b"\r\n");

        let suffix_len = if is_last_chunk { 7 } else { 2 };

        EncodedChunk {
            data,
            suffix: &SUFFIX[..suffix_len],
        }
    }
}
