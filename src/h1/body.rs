// rfc-editor.org/rfc/rfc9110.html#name-representation-data-and-met
//
// Content-Type - with boundary for multipart
// Content-Encoding - gzip, deflate, brotli
// Content-Length
// Transfer-Encoding - chunked, gzip, etc.

use std::task::Poll;
use tcio::bytes::BytesMut;
use tcio::num::wrapping_atou;

use crate::body::Incoming;
use crate::body::error::BodyError;
use crate::body::shared::{BodyDecoder, SendHandle};
use crate::h1::chunked::ChunkedCoder;
use crate::headers::HeaderMap;
use crate::headers::standard::{CONTENT_LENGTH, TRANSFER_ENCODING};

#[derive(Debug)]
pub struct H1BodyDecoder {
    kind: BodyKind,
}

#[derive(Clone, Debug)]
pub enum BodyKind {
    ContentLength(u64),
    Chunked(ChunkedCoder),
}

impl H1BodyDecoder {
    pub fn new(headers: &HeaderMap) -> Result<Self, BodyError> {
        let mut content_lengths = headers.get_all(&CONTENT_LENGTH);
        let mut transfer_encodings = headers.get_all(&TRANSFER_ENCODING);

        let kind = match (content_lengths.next(), transfer_encodings.has_remaining()) {
            (None, false) => BodyKind::ContentLength(0),
            (None, true) => {
                // TODO: support compressed transfer-encodings

                let ok = transfer_encodings.all(|e|e.as_bytes().eq_ignore_ascii_case(b"chunked"));
                if !ok {
                    return Err(BodyError::UnknownCodings);
                }

                BodyKind::Chunked(ChunkedCoder::new())
            }
            (Some(length), false) => {
                if content_lengths.next().is_some() {
                    return Err(BodyError::InvalidContentLength);
                }
                match wrapping_atou(length.as_bytes()) {
                    Some(length) => BodyKind::ContentLength(length),
                    None => return Err(BodyError::InvalidContentLength),
                }
            }
            (Some(_), true) => return Err(BodyError::InvalidCodings),
        };
        Ok(Self { kind })
    }

    pub fn has_remaining(&self) -> bool {
        match &self.kind {
            BodyKind::Chunked(chunked) => !chunked.is_eof(),
            BodyKind::ContentLength(len) => *len != 0,
        }
    }

    pub fn remaining(&self) -> Option<u64> {
        match self.kind {
            BodyKind::Chunked(_) => None,
            BodyKind::ContentLength(len) => Some(len),
        }
    }

    pub fn build_body(
        &self,
        buffer: &mut BytesMut,
        shared: &mut SendHandle,
        cx: &mut std::task::Context,
    ) -> Incoming {
        match self.kind {
            BodyKind::ContentLength(0) => Incoming::empty(),
            BodyKind::Chunked(_) => Incoming::from_handle(shared.handle(cx), None),
            BodyKind::ContentLength(len) => {
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
            BodyKind::Chunked(decoder) => decoder.decode_chunk(buffer),
            BodyKind::ContentLength(0) => Poll::Ready(None),
            BodyKind::ContentLength(remaining_mut) => {
                if buffer.is_empty() {
                    return Poll::Pending;
                }
                let cnt = (*remaining_mut).min(buffer.len() as u64);
                *remaining_mut -= cnt;
                Poll::Ready(Some(Ok(buffer.split_to(cnt as usize))))
            }
        }
    }
}

impl BodyDecoder for H1BodyDecoder {
    fn decode_chunk(
        &mut self,
        read_buffer: &mut BytesMut,
    ) -> Poll<Result<Option<BytesMut>, BodyError>> {
        self.decode_chunk(read_buffer)
            .map(|result| result.transpose())
    }

    fn can_drain(&self) -> bool {
        const MIN_BODY_DRAIN: u64 = 64 * 1024;
        self.remaining().unwrap_or(u64::MAX) <= MIN_BODY_DRAIN
    }

    fn poll_drain(&mut self, read_buffer: &mut BytesMut) -> Poll<Result<(), BodyError>> {
        while self.has_remaining() {
            match std::task::ready!(BodyDecoder::decode_chunk(self, read_buffer)) {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(err) => return Poll::Ready(Err(err)),
            }
        }
        Poll::Ready(Ok(()))
    }
}

