use tcio::bytes::BytesMut;
use std::{
    io,
    task::{Poll, ready},
};

pub use crate::h1::io::IoHandle;

#[derive(Debug)]
pub struct BodyHandle {
    handle: IoHandle,
    remaining: u64,
}

impl BodyHandle {
    pub fn new(handle: IoHandle, remaining: u64) -> Self {
        Self {
            handle,
            remaining,
        }
    }

    pub fn remaining(&self) -> usize {
        self.remaining as usize
    }

    pub fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }

    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<BytesMut>> {
        let data = ready!(self.handle.poll_read(cx)?);

        self.remaining -= u64::try_from(data.len()).unwrap_or(u64::MAX);

        Poll::Ready(Ok(data))
    }
}
