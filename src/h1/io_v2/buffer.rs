use std::io;
use std::task::{Context, Poll};
use tcio::bytes::BytesMut;
use tcio::io::{AsyncIoRead, AsyncIoWrite};

#[derive(Debug)]
pub struct IoBuffer<IO> {
    io: IO,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
}

impl<IO> IoBuffer<IO> {
    pub fn new(io: IO) -> Self {
        Self {
            io,
            read_buffer: BytesMut::with_capacity(512),
            write_buffer: BytesMut::with_capacity(512),
        }
    }

    pub fn read_buffer_mut(&mut self) -> &mut BytesMut {
        &mut self.read_buffer
    }

    pub fn write_buffer_mut(&mut self) -> &mut BytesMut {
        &mut self.write_buffer
    }
}

impl<IO> IoBuffer<IO>
where
    IO: AsyncIoRead,
{
    pub fn poll_read(&mut self, cx: &mut Context) -> Poll<io::Result<usize>> {
        self.io.poll_read_buf(&mut self.read_buffer, cx)
    }
}

impl<IO> IoBuffer<IO>
where
    IO: AsyncIoWrite,
{
    pub fn poll_write(&mut self, cx: &mut Context) -> Poll<io::Result<usize>> {
        self.io.poll_write_buf(&mut self.write_buffer, cx)
    }
}
