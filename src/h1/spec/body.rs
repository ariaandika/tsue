use std::fmt;
use std::task::Poll;
use tcio::bytes::BytesMut;

use super::ProtoError;
use super::chunked::ChunkedDecoder;
use crate::headers::HeaderMap;
use crate::headers::standard::{CONTENT_LENGTH, TRANSFER_ENCODING};

#[derive(Debug)]
pub struct MessageBody {
    coding: Coding,
}

#[derive(Clone, Debug)]
pub enum Coding {
    Empty,
    Chunked(ChunkedDecoder),
    ContentLength(u64),
}

impl MessageBody {
    pub fn new(headers: &HeaderMap) -> Result<Self, ProtoError> {
        let mut content_lengths = headers.get_all(CONTENT_LENGTH);
        let mut transfer_encodings = headers.get_all(TRANSFER_ENCODING);

        let coding = match (content_lengths.next(), transfer_encodings.has_remaining()) {
            (None, false) => Coding::ContentLength(0),
            (None, true) => {
                // TODO: support compressed transfer-encodings

                let ok = transfer_encodings.all(|e|e.as_bytes().eq_ignore_ascii_case(b"chunked"));
                if !ok {
                    return Err(ProtoError::UnknownCodings);
                }

                Coding::Chunked(ChunkedDecoder::new())
            }
            (Some(length), false) => {
                if content_lengths.has_remaining() {
                    return Err(ProtoError::InvalidContentLength);
                }
                match tcio::atou(length.as_bytes()) {
                    Some(length) => Coding::ContentLength(length),
                    None => return Err(ProtoError::InvalidContentLength),
                }
            }
            (Some(_), true) => return Err(ProtoError::InvalidCodings),
        };
        Ok(Self { coding })
    }

    /// Returns Poll::Pending if more data read is required.
    pub(crate) fn poll_read(&mut self, buffer: &mut BytesMut) -> Poll<Result<Option<BytesMut>, BodyError>> {
        match &mut self.coding {
            Coding::Empty => Poll::Ready(Err(BodyError::Exhausted)),
            Coding::Chunked(decoder) => decoder.poll_chunk(buffer),
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
                        Poll::Ready(Ok(Some(buffer.split_to(remaining as usize))))
                    }
                    // buffer does not contains all expected content
                    Some(leftover) => {
                        *remaining_mut = leftover;
                        Poll::Ready(Ok(Some(buffer.split())))
                    }
                }
            }
        }
    }
}

// ===== Error =====

/// A semantic error when reading message body.
#[derive(Debug)]
pub enum BodyError {
    /// User error where it tries to read empty body.
    Exhausted,
    /// Client error where chunked format is invalid.
    InvalidChunked,
    /// Client error where chunked length is too large.
    ChunkTooLarge,
}

impl BodyError {
    const fn message(&self) -> &'static str {
        match self {
            Self::Exhausted => "message body exhausted",
            Self::InvalidChunked => "invalid chunked format",
            Self::ChunkTooLarge => "chunk too large",
        }
    }
}

impl std::error::Error for BodyError { }

impl fmt::Display for BodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

