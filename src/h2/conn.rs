use std::task::Poll;
use std::{pin::Pin, task::ready};
use tcio::bytes::BytesMut;
use tcio::io::{AsyncRead, AsyncWrite};

use crate::h2::state::H2State;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const DEFAULT_BUFFER_CAP: usize = 512;

macro_rules! io_read {
    ($read:expr) => {
        let read = ready!($read)?;
        if read == 0 {
            return Poll::Ready(Ok(()));
        }
    };
}

/// HTTP/2 Connection.
#[derive(Debug)]
pub struct Connection<IO> {
    io: IO,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    /// will be `None` pre-preface
    state: Option<H2State>,
}

type ConnectionProject<'a, IO> = (
    Pin<&'a mut IO>,
    &'a mut BytesMut,
    &'a mut BytesMut,
    &'a mut Option<H2State>,
);

impl<IO> Connection<IO> {
    pub fn new(io: IO) -> Self {
        Self {
            io,
            read_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            write_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            state: None,
        }
    }
}

impl<IO> Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    fn try_poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> Poll<Result<(), BoxError>> {
        let (mut io, read_buffer, write_buffer, state) = self.as_mut().project();

        let state = loop {
            match state {
                Some(ok) => break ok,
                None => match H2State::preface_chunk(&mut *read_buffer) {
                    Poll::Ready(result) => break state.insert(result?),
                    Poll::Pending => {
                        io_read!(io.as_mut().poll_read(&mut *read_buffer, cx));
                        continue;
                    }
                },
            }
        };

        loop {
            if state.poll_frame(read_buffer, write_buffer)?.is_none()
                && ready!(io.as_mut().poll_read(&mut *read_buffer, cx)?) == 0
            {
                return Poll::Ready(Ok(()));
            }
        }
    }
}

impl<IO> Future for Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        if let Err(err) = ready!(self.try_poll(cx)) {
            eprintln!("[ERROR] {err}")
        }
        Poll::Ready(())
    }
}

// ===== Projection =====

impl<IO> Connection<IO> {
    fn project(self: Pin<&mut Self>) -> ConnectionProject<'_, IO> {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe {
            let me = self.get_unchecked_mut();
            (
                Pin::new_unchecked(&mut me.io),
                &mut me.read_buffer,
                &mut me.write_buffer,
                &mut me.state,
            )
        }
    }
}

