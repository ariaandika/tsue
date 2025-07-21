use bytes::BytesMut;
use std::{
    io,
    task::{Poll, ready},
};

pub use crate::proto::h1::IoHandle;

#[derive(Debug)]
pub struct BodyHandle {
    handle: IoHandle,
    remaining: u64,
    remain: BytesMut,
}

impl BodyHandle {
    pub fn new(handle: IoHandle, remaining: u64, remain: BytesMut) -> Self {
        debug_assert!(remaining as usize >= remain.len());
        Self {
            handle,
            remaining,
            remain,
        }
    }

    pub fn remaining(&self) -> usize {
        self.remain.len() + self.remaining as usize
    }

    pub fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }

    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<BytesMut>> {
        if !self.remain.is_empty() {
            self.remaining -= u64::try_from(self.remain.len()).unwrap_or(u64::MAX);
            return Poll::Ready(Ok(std::mem::take(&mut self.remain)));
        }

        let data = ready!(self.handle.poll_read(cx)?);

        self.remaining -= u64::try_from(data.len()).unwrap_or(u64::MAX);

        Poll::Ready(Ok(data))
    }
}
