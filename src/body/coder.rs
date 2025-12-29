// rfc-editor.org/rfc/rfc9110.html#name-representation-data-and-met
//
// Content-Type - with boundary for multipart
// Content-Encoding - gzip, deflate, brotli
// Content-Length
// Transfer-Encoding - chunked, gzip, etc.

use std::task::{Poll, ready};
use tcio::bytes::{Bytes, BytesMut};
use tcio::io::AsyncIoWrite;

use crate::body::chunked::ChunkedCoder;
use crate::body::error::{BodyError, ReadError};
use crate::body::handle::Shared;
use crate::body::{Codec, Incoming};
use crate::headers::HeaderMap;
use crate::headers::standard::{CONTENT_LENGTH, TRANSFER_ENCODING};

#[derive(Debug)]
pub struct BodyCoder {
    kind: Kind,
}

#[derive(Clone, Debug)]
enum Kind {
    Chunked(ChunkedCoder),
    ContentLength(u64),
}

impl BodyCoder {
    pub fn from_len(len: Option<u64>) -> Self {
        let kind = match len {
            Some(len) => Kind::ContentLength(len),
            None => Kind::Chunked(ChunkedCoder::new()),
        };
        Self {
            kind,
        }
    }

    pub fn new(headers: &HeaderMap) -> Result<Self, BodyError> {
        let mut content_lengths = headers.get_all(CONTENT_LENGTH);
        let mut transfer_encodings = headers.get_all(TRANSFER_ENCODING);

        let kind = match (content_lengths.next(), transfer_encodings.has_remaining()) {
            (None, false) => Kind::ContentLength(0),
            (None, true) => {
                // TODO: support compressed transfer-encodings

                let ok = transfer_encodings.all(|e|e.as_bytes().eq_ignore_ascii_case(b"chunked"));
                if !ok {
                    return Err(BodyError::UnknownCodings);
                }

                Kind::Chunked(ChunkedCoder::new())
            }
            (Some(length), false) => {
                if content_lengths.has_remaining() {
                    return Err(BodyError::InvalidContentLength);
                }
                match tcio::atou(length.as_bytes()) {
                    Some(length) => Kind::ContentLength(length),
                    None => return Err(BodyError::InvalidContentLength),
                }
            }
            (Some(_), true) => return Err(BodyError::InvalidCodings),
        };
        Ok(Self { kind })
    }

    pub fn build_body(
        &self,
        buffer: &mut BytesMut,
        shared: &mut Shared,
        cx: &mut std::task::Context,
    ) -> Incoming {
        match &self.kind {
            Kind::ContentLength(0) => Incoming::empty(),
            Kind::Chunked(_) => Incoming::from_handle(shared.handle(cx), None),
            Kind::ContentLength(len) => {
                if buffer.len() as u64 == *len {
                    Incoming::new(buffer.split())
                } else {
                    Incoming::from_handle(shared.handle(cx), Some(*len))
                }
            }
        }
    }

    /// Returns Poll::Pending if more data read is required.
    pub(crate) fn decode_chunk(
        &mut self,
        buffer: &mut BytesMut,
    ) -> Poll<Option<Result<BytesMut, BodyError>>> {
        match &mut self.kind {
            Kind::Chunked(decoder) => decoder.decode_chunk(buffer),
            Kind::ContentLength(remaining_mut) => {
                let remaining = *remaining_mut;
                match remaining.checked_sub(buffer.len() as u64) {
                    // buffer contains exact or larger than expected content
                    None | Some(0) => {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "remaining <= buffer.len() which is usize"
                        )]
                        Poll::Ready(Some(Ok(buffer.split_to(remaining as usize))))
                    }
                    // buffer does not contains all expected content
                    Some(leftover) => {
                        *remaining_mut = leftover;
                        Poll::Ready(Some(Ok(buffer.split())))
                    }
                }
            }
        }
    }

    pub(crate) fn encode_chunk<W: AsyncIoWrite>(
        &mut self,
        chunk: &mut Bytes,
        write_buffer: &mut BytesMut,
        io: &mut W,
        cx: &mut std::task::Context,
    ) -> Poll<Result<(), ReadError>> {
        match &mut self.kind {
            Kind::Chunked(decoder) => decoder.encode_chunk(chunk, write_buffer, io, cx),
            Kind::ContentLength(remaining_mut) => {
                if *remaining_mut < chunk.len() as u64 {
                    return Poll::Ready(Err(BodyError::InvalidSizeHint.into()));
                }
                let read = ready!(io.poll_write_buf(chunk, cx))?;
                *remaining_mut -= read as u64;
                Poll::Ready(Ok(()))
            },
        }
    }

    pub const fn coding(&self) -> Codec {
        match &self.kind {
            Kind::Chunked(_) => Codec::Chunked,
            Kind::ContentLength(len) => Codec::ContentLength(*len),
        }
    }
}

