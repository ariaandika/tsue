use std::task::{Poll, ready};
use tcio::bytes::Bytes;

use crate::body::error::ReadError;
use crate::body::shared::RecvHandle;

#[derive(Debug)]
pub struct BodyHandle {
    handle: RecvHandle,
    size_hint: Option<u64>,
}

impl BodyHandle {
    pub fn new(handle: RecvHandle, size_hint: Option<u64>) -> Self {
        Self { handle, size_hint }
    }

    pub const fn size_hint(&self) -> Option<u64> {
        self.size_hint
    }

    pub fn poll_read(
        &mut self,
        cx: &mut std::task::Context,
    ) -> Poll<Option<Result<Bytes, ReadError>>> {
        let Some(data) = ready!(self.handle.poll_read(cx)?) else {
            return Poll::Ready(None);
        };

        if let Some(size_hint) = &mut self.size_hint {
            *size_hint -= data.len() as u64;
        }

        Poll::Ready(Some(Ok(data)))
    }

    pub const fn is_end_stream(&self) -> bool {
        match self.size_hint {
            Some(len) => len == 0,
            None => false,
        }
    }
}
