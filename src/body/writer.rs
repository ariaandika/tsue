use std::{
    io,
    task::{Poll, ready},
};
use tcio::{bytes::Bytes, io::AsyncIoWrite};

use super::Incoming;
use crate::h1::io::IoBuffer;

pub struct BodyWrite {
    body: Incoming,
    phase: Phase,
}

enum Phase {
    Read,
    Write(Bytes),
}

impl BodyWrite {
    pub fn new(body: Incoming) -> Self {
        Self {
            body,
            phase: Phase::Read,
        }
    }

    pub fn poll_write<IO: AsyncIoWrite>(
        &mut self,
        io: &mut IoBuffer<IO>,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<()>> {
        // loop {
        //     match &mut self.phase {
        //         Phase::Read => {
        //             if self.body.has_remaining() {
        //                 let data = ready!(self.body.poll_read(cx))?;
        //                 self.phase = Phase::Write(data);
        //             } else {
        //                 break;
        //             }
        //         }
        //         Phase::Write(bytes) => {
        //             ready!(io.poll_write(bytes, cx))?;
        //             bytes.clear();
        //             if bytes.is_empty() {
        //                 self.phase = Phase::Read;
        //             }
        //         }
        //     }
        // }
        // Poll::Ready(Ok(()))
        todo!()
    }
}

