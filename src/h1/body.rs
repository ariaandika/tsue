// rfc-editor.org/rfc/rfc9110.html#name-representation-data-and-met
//
// Content-Type - with boundary for multipart
// Content-Encoding - gzip, deflate, brotli
// Content-Length
// Transfer-Encoding - chunked, gzip, etc.

use std::task::Poll;
use tcio::bytes::BytesMut;

use crate::body::Incoming;
use crate::body::error::BodyError;
use crate::body::shared::{BodyDecode, SendHandle};
use crate::h1::chunked::ChunkedCoder;
use crate::proto::error::UserError;

const MIN_BODY_DRAIN: u64 = 64 * 1024;

pub enum ContentKind {
    ContentLength(u64),
    Chunked,
}

// ===== Decoder =====

pub struct BodyDecoder {
    kind: DecoderKind,
}

enum DecoderKind {
    Length(u64),
    Chunked(ChunkedCoder),
}

impl BodyDecoder {
    pub fn new(kind: ContentKind) -> Self {
        let kind = match kind {
            ContentKind::ContentLength(len) => DecoderKind::Length(len),
            ContentKind::Chunked => DecoderKind::Chunked(ChunkedCoder::new())
        };
        Self { kind }
    }
}

impl BodyDecode for BodyDecoder {
    fn decode_chunk(
        &mut self,
        read_buffer: &mut BytesMut,
    ) -> Poll<Result<Option<BytesMut>, BodyError>> {
        self.decode_chunk(read_buffer)
            .map(|result| result.transpose())
    }
}

impl BodyDecoder {
    pub fn build_body(
        &self,
        buffer: &mut BytesMut,
        shared: &mut SendHandle,
        cx: &mut std::task::Context,
    ) -> Incoming {
        match self.kind {
            DecoderKind::Length(0) => Incoming::empty(),
            DecoderKind::Length(len) => {
                if buffer.len() as u64 == len {
                    Incoming::new(buffer.split())
                } else {
                    Incoming::from_handle(shared.handle(cx), Some(len))
                }
            }
            DecoderKind::Chunked(_) => Incoming::from_handle(shared.handle(cx), None),
        }
    }

    /// Returns Poll::Pending if more data read is required.
    pub fn decode_chunk(
        &mut self,
        buffer: &mut BytesMut,
    ) -> Poll<Option<Result<BytesMut, BodyError>>> {
        match &mut self.kind {
            DecoderKind::Length(0) => Poll::Ready(None),
            DecoderKind::Length(remaining_mut) => {
                if buffer.is_empty() {
                    return Poll::Pending;
                }
                let cnt = (*remaining_mut).min(buffer.len() as u64);
                *remaining_mut -= cnt;
                Poll::Ready(Some(Ok(buffer.split_to(cnt as usize))))
            }
            DecoderKind::Chunked(decoder) => decoder.decode_chunk(buffer),
        }
    }

    /// Returns `Ok(bool)` indicating whether message body draining is required.
    ///
    /// # Errors
    ///
    /// Returns error if message body draining is unable to be performed.
    pub fn needs_drain(&self) -> Result<bool, UserError> {
        let DecoderKind::Length(len) = self.kind else {
            return Err(UserError::UnreadRequestContent);
        };
        if len > MIN_BODY_DRAIN {
            return Err(UserError::UnreadRequestContent);
        }
        Ok(len != 0)
    }

    pub fn poll_drain(&mut self, read: usize) -> Poll<()> {
        let DecoderKind::Length(remain_mut) = &mut self.kind else {
            unreachable!("chunked encoding cannot be drained");
        };
        *remain_mut = remain_mut.saturating_sub(read as u64);
        if *remain_mut == 0 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

// ===== Encoder =====

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub enum BodyEncoder {
    Length(LengthEncoder),
    Chunked(ChunkedCoder),
}

impl BodyEncoder {
    pub fn new_length(remaining: u64) -> Self {
        Self::Length(LengthEncoder { remaining })
    }

    pub fn new_chunked() -> Self {
        Self::Chunked(ChunkedCoder::new())
    }
}

pub struct LengthEncoder {
    remaining: u64,
}

impl LengthEncoder {
    pub fn has_remaining(&self) -> bool {
        self.remaining != 0
    }

    pub fn encode<D>(&mut self, data: crate::body::Frame<D>) -> Result<D, BoxError>
    where
        D: tcio::bytes::Buf,
    {
        match data.into_data() {
            Ok(data) => match self.remaining.checked_sub(data.remaining() as u64) {
                Some(remain) => {
                    self.remaining = remain;
                    Ok(data)
                }
                None => Err("user content larger than given size hint".into()),
            },
            // NOTE: currently trailer from user are dropped
            Err(_) => todo!(),
        }
    }
}
