// rfc-editor.org/rfc/rfc9110.html#name-representation-data-and-met
//
// Content-Type - with boundary for multipart
// Content-Encoding - gzip, deflate, brotli
// Content-Length
// Transfer-Encoding - chunked, gzip, etc.

use std::task::Poll;
use tcio::bytes::{Bytes, BytesMut};
use tcio::io::AsyncIoWrite;

use crate::body::chunked::ChunkedDecoder;
use crate::headers::HeaderMap;
use crate::headers::standard::{CONTENT_LENGTH, TRANSFER_ENCODING};
use crate::body::Incoming;
use crate::body::handle::Shared;
use crate::body::error::BodyError;

#[derive(Debug)]
pub struct BodyDecoder {
    coding: Coding,
}

#[derive(Clone, Debug)]
pub enum Coding {
    /// TODO: Currently, content-length: 0, and body exhausted state is separated
    Empty,
    Chunked(ChunkedDecoder),
    ContentLength(u64),
}

impl BodyDecoder {
    pub fn from_len(len: Option<u64>) -> Self {
        let coding = match len {
            Some(len) => Coding::ContentLength(len),
            None => Coding::Chunked(ChunkedDecoder::new()),
        };
        Self {
            coding,
        }
    }

    pub fn new(headers: &HeaderMap) -> Result<Self, BodyError> {
        let mut content_lengths = headers.get_all(CONTENT_LENGTH);
        let mut transfer_encodings = headers.get_all(TRANSFER_ENCODING);

        let coding = match (content_lengths.next(), transfer_encodings.has_remaining()) {
            (None, false) => Coding::ContentLength(0),
            (None, true) => {
                // TODO: support compressed transfer-encodings

                let ok = transfer_encodings.all(|e|e.as_bytes().eq_ignore_ascii_case(b"chunked"));
                if !ok {
                    return Err(BodyError::UnknownCodings);
                }

                Coding::Chunked(ChunkedDecoder::new())
            }
            (Some(length), false) => {
                if content_lengths.has_remaining() {
                    return Err(BodyError::InvalidContentLength);
                }
                match tcio::atou(length.as_bytes()) {
                    Some(length) => Coding::ContentLength(length),
                    None => return Err(BodyError::InvalidContentLength),
                }
            }
            (Some(_), true) => return Err(BodyError::InvalidCodings),
        };
        Ok(Self { coding })
    }

    pub fn build_body(
        &self,
        buffer: &mut BytesMut,
        shared: &mut Shared,
        cx: &mut std::task::Context,
    ) -> Incoming {
        match &self.coding {
            Coding::Empty | Coding::ContentLength(0) => Incoming::empty(),
            Coding::Chunked(_) => Incoming::from_handle(shared.handle(cx), None),
            Coding::ContentLength(len) => {
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
        match &mut self.coding {
            Coding::Empty => Poll::Ready(Some(Err(BodyError::Exhausted))),
            Coding::Chunked(decoder) => decoder.decode_chunk(buffer),
            Coding::ContentLength(remaining_mut) => {
                let remaining = *remaining_mut;
                match remaining.checked_sub(buffer.len() as u64) {
                    // buffer contains exact or larger than expected content
                    None | Some(0) => {
                        self.coding = Coding::Empty;
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
        mut chunk: Bytes,
        io: &mut W,
    ) -> Poll<Result<(), BodyError>> {
        match &mut self.coding {
            Coding::Chunked(decoder) => decoder.encode_chunk(io),
            Coding::ContentLength(remaining_mut) => {
                let remaining = *remaining_mut;
                match remaining.checked_sub(chunk.len() as u64) {
                    // chunk contains exact or larger than expected content
                    None | Some(0) => {
                        self.coding = Coding::Empty;
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "remaining <= buffer.len() which is usize"
                        )]
                        chunk.truncate(remaining as usize);
                        todo!("statefull chunk encoder")
                        // Poll::Ready(Ok(()))
                    }
                    // buffer does not contains all expected content
                    Some(leftover) => {
                        *remaining_mut = leftover;
                        // Poll::Ready(Some(Ok(buffer.split())))
                        todo!("statefull chunk encoder")
                    }
                }
            },
            Coding::Empty => Poll::Ready(Ok(()))
        }
    }

    pub const fn coding(&self) -> &Coding {
        &self.coding
    }
}

