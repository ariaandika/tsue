use tcio::bytes::{Buf, Bytes};
use std::{io, pin::Pin, task::Poll};

pub trait BodyStream {
    type Data: Buf;

    type Error;

    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> Poll<Result<Self::Data, Self::Error>>;

    fn has_remaining(&self) -> bool;

    fn size_hint(&self) -> (usize, Option<usize>);
}

#[derive(Debug)]
pub struct BodyStreamHandle {}

impl BodyStreamHandle {
    pub fn new<S>(_stream: S) -> Self
    where
        S: BodyStream
    {
        todo!()
    }

    pub fn remaining(&self) -> usize {
        todo!()
    }

    pub fn poll_read(&mut self, _cx: &mut std::task::Context) -> Poll<io::Result<Bytes>> {
        todo!()
    }

    pub fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }
}
