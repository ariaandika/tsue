use tcio::bytes::Bytes;
use std::{
    io,
    task::{Poll, ready},
};

mod shared;

pub use shared::{Shared, Handle as IoHandle};

#[derive(Debug)]
pub struct BodyHandle {
    handle: IoHandle,
    size_hint: Option<u64>,
}

impl BodyHandle {
    pub fn new(handle: IoHandle, size_hint: Option<u64>) -> Self {
        Self {
            handle,
            size_hint,
        }
    }

    pub const fn size_hint(&self) -> Option<u64> {
        self.size_hint
    }

    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<Option<io::Result<Bytes>>> {
        let data = ready!(self.handle.poll_read(cx)?);

        if let Some(size_hint) = &mut self.size_hint {
            *size_hint -= data.len() as u64;
        }

        Poll::Ready(Some(Ok(data)))
    }
}
