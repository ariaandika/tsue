// rfc-editor.org/rfc/rfc9110.html#name-representation-data-and-met
//
// Content-Type - with boundary for multipart
// Content-Encoding - gzip, deflate, brotli
// Content-Length
// Transfer-Encoding - chunked, gzip, etc.

use std::task::Poll;
use tcio::bytes::{Buf, BytesMut};
use tcio::num::atou;

use crate::body::chunked::{ChunkedCoder, EncodedBuf};
use crate::body::error::BodyError;
use crate::body::handle::SendHandle;
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
        Self {
            kind: match len {
                Some(len) => Kind::ContentLength(len),
                None => Kind::Chunked(ChunkedCoder::new()),
            },
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
                match atou(length.as_bytes()) {
                    Some(length) => Kind::ContentLength(length),
                    None => return Err(BodyError::InvalidContentLength),
                }
            }
            (Some(_), true) => return Err(BodyError::InvalidCodings),
        };
        Ok(Self { kind })
    }

    pub fn has_remaining(&self) -> bool {
        match self.kind {
            Kind::Chunked(_) => true,
            Kind::ContentLength(len) => len != 0,
        }
    }

    pub fn size_hint(&self) -> Option<u64> {
        match self.kind {
            Kind::Chunked(_) => None,
            Kind::ContentLength(len) => Some(len),
        }
    }

    pub fn build_body(
        &self,
        buffer: &mut BytesMut,
        shared: &mut SendHandle,
        cx: &mut std::task::Context,
    ) -> Incoming {
        match self.kind {
            Kind::ContentLength(0) => Incoming::empty(),
            Kind::Chunked(_) => Incoming::from_handle(shared.handle(cx), None),
            Kind::ContentLength(len) => {
                if buffer.len() as u64 == len {
                    Incoming::new(buffer.split())
                } else {
                    Incoming::from_handle(shared.handle(cx), Some(len))
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
            Kind::ContentLength(0) => Poll::Ready(None),
            Kind::ContentLength(remaining_mut) => {
                if buffer.is_empty() {
                    return Poll::Pending;
                }
                let cnt = (*remaining_mut).min(buffer.len() as u64);
                *remaining_mut -= cnt;
                Poll::Ready(Some(Ok(buffer.split_to(cnt as usize))))
            }
        }
    }

    /// Encode message body chunk.
    ///
    /// Returns [`EncodedBuf`] which is just a bytes that also may contains chunk header.
    pub(crate) fn encode_chunk<B: Buf>(
        &mut self,
        chunk: B,
        write_buffer: &mut BytesMut,
        is_last_chunk: bool,
    ) -> Result<EncodedBuf<B>, BodyError> {
        match &mut self.kind {
            Kind::Chunked(decoder) => Ok(decoder.encode_chunk(chunk, write_buffer, is_last_chunk)),
            Kind::ContentLength(remaining_mut) => {
                match remaining_mut.checked_sub(chunk.remaining() as u64) {
                    Some(rem) => {
                        *remaining_mut = rem;
                        Ok(EncodedBuf::exact(chunk))
                    },
                    None => Err(BodyError::InvalidSizeHint),
                }
            }
        }
    }

    pub const fn coding(&self) -> Codec {
        match self.kind {
            Kind::Chunked(_) => Codec::Chunked,
            Kind::ContentLength(len) => Codec::ContentLength(len),
        }
    }
}

