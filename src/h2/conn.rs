use std::task::Poll;
use std::{pin::Pin, task::ready};
use tcio::bytes::BytesMut;
use tcio::io::{AsyncRead, AsyncWrite};

use crate::h2::state::{FrameResult, H2State};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const DEFAULT_BUFFER_CAP: usize = 512;

/// HTTP/2 Connection.
#[derive(Debug)]
pub struct Connection<IO> {
    io: IO,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    /// will be `None` pre-preface
    phase: Phase,
}

#[derive(Debug)]
enum Phase {
    Connection(H2State),
    Handshake,
}

type ConnectionProject<'a, IO> = (
    Pin<&'a mut IO>,
    &'a mut BytesMut,
    &'a mut BytesMut,
    &'a mut Phase,
);

impl<IO> Connection<IO> {
    pub fn new(io: IO) -> Self {
        Self {
            io,
            read_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            write_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            phase: Phase::Handshake,
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
    ) -> Poll<Result<bool, BoxError>> {
        let (mut io, read_buffer, write_buffer, state) = self.as_mut().project();

        match state {
            Phase::Handshake => {
                if let Poll::Ready(result) =
                    H2State::handshake(&mut *read_buffer, &mut *write_buffer)
                {
                    *state = Phase::Connection(result?);
                }
            }
            Phase::Connection(state) => {
                while let Some(result) = state.poll_frame(read_buffer, write_buffer)? {
                    match result {
                        FrameResult::None => {}
                        FrameResult::Request(_stream_id, _map) => todo!(),
                        FrameResult::Data(_stream_id, _data) => todo!(),
                        FrameResult::Shutdown => todo!(),
                    }
                }
            }
        }

        let _ = io.as_mut().poll_write_all_buf(&mut *write_buffer, cx)?;
        let result = io.as_mut().poll_read(&mut *read_buffer, cx)?;
        Poll::Ready(Ok(matches!(result, Poll::Ready(0))))
    }
}

impl<IO> Future for Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        loop {
            match ready!(self.as_mut().try_poll(cx)) {
                Ok(true) => { }
                Ok(false) => {
                    println!("[CONNECTION] Closed");
                    break;
                },
                Err(err) => {
                    eprintln!("[ERROR] {err}");
                    break;
                }
            }
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
                &mut me.phase,
            )
        }
    }
}

