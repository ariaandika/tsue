use std::{num::NonZeroU64, task::Poll};
use tcio::bytes::{Buf, BytesMut};

use crate::body::error::BodyError;

const MAX_CHUNKED_SIZE: u64 = 64 * 1024;

#[derive(Clone, Debug)]
pub struct ChunkedCoder {
    phase: Phase,
}

#[derive(Clone, Debug)]
enum Phase {
    Header,
    Chunk(NonZeroU64),
}

impl ChunkedCoder {
    pub(crate) fn new() -> Self {
        Self {
            phase: Phase::Header,
        }
    }

    /// Poll for chunked body, returns `None` it end of chunks found.
    pub(crate) fn decode_chunk(
        &mut self,
        buffer: &mut BytesMut,
    ) -> Poll<Option<Result<BytesMut, BodyError>>> {
        match &mut self.phase {
            Phase::Header => {
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
                buffer.advance(digits_len + trailing_header);

                match NonZeroU64::new(chunk_len) {
                    Some(nonzero_len) => {
                        self.phase = Phase::Chunk(nonzero_len);
                        // advance
                        self.decode_chunk(buffer)
                    }
                    None => match buffer.first_chunk::<2>() {
                        Some(b"\r\n") => {
                            buffer.advance(2);
                            Poll::Ready(None)
                        }
                        Some(_) => Poll::Ready(Some(Err(BodyError::InvalidChunked))),
                        None => Poll::Pending,
                    }
                }
            }
            Phase::Chunk(remaining_mut) => {
                let remaining = remaining_mut.get();
                match remaining
                    .checked_sub(buffer.len() as u64)
                    .and_then(NonZeroU64::new)
                {
                    // buffer contains partial of the expected chunk
                    Some(leftover) => {
                        *remaining_mut = leftover;
                        Poll::Ready(Some(Ok(buffer.split())))
                    }
                    // buffer contains exact or larger than expected content
                    None => {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "remaining <= buffer.len() which is usize"
                        )]
                        let remaining = remaining as usize;
                        let body = buffer.split_to(remaining);
                        match buffer.first_chunk::<2>() {
                            Some(b"\r\n") => {
                                self.phase = Phase::Header;
                                buffer.advance(2);
                            }
                            Some(_) => return Poll::Ready(Some(Err(BodyError::InvalidChunked))),
                            None => return Poll::Pending,
                        }
                        Poll::Ready(Some(Ok(body)))
                    }
                }
            }
        }
    }

    pub fn encode_chunk<W: tcio::io::AsyncIoWrite>(
        &mut self,
        io: &mut W,
    ) -> Poll<Result<(), BodyError>> {
        todo!()
    }
}
