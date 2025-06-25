use bytes::BytesMut;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::Body;

/// A future returned from [`Body::collect`], resolved to the entire body.
#[derive(Debug)]
pub struct Collect {
    body: Body,
    buffer: Option<BytesMut>,
}

impl Collect {
    pub(crate) fn new(body: Body) -> Self {
        let buffer = BytesMut::with_capacity(body.remaining());
        Self {
            body,
            buffer: Some(buffer),
        }
    }
}

impl Future for Collect {
    type Output = io::Result<BytesMut>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();
        let buffer = me.buffer.as_mut().expect("poll after complete");

        while me.body.has_remaining() {
            ready!(me.body.poll_read_buf(buffer, cx)?);
        }

        Poll::Ready(Ok(me.buffer.take().expect("poll after complete")))
    }
}

