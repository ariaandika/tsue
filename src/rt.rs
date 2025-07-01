use std::{
    io,
    mem::MaybeUninit,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Poll, ready},
};
use bytes::Bytes;
use tcio::io::{AsyncBufRead, AsyncIoRead, AsyncIoWrite, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

use crate::{
    request::{
        Request,
        parser::{parse_headers_uninit, parse_line},
    },
    service::Service,
};

// ===== Listener =====

pub trait Listener {
    type Stream: AsyncIoRead + AsyncIoWrite;

    type Addr;

    fn poll_accept(
        &self,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<(Self::Stream, Self::Addr)>>;
}

impl Listener for TcpListener {
    type Stream = TcpStream;

    type Addr = SocketAddr;

    fn poll_accept(
        &self,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<(Self::Stream, Self::Addr)>> {
        TcpListener::poll_accept(self, cx)
    }
}

#[cfg(unix)]
impl Listener for UnixListener {
    type Stream = UnixStream;

    type Addr = tokio::net::unix::SocketAddr;

    fn poll_accept(
        &self,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<(Self::Stream, Self::Addr)>> {
        UnixListener::poll_accept(self, cx)
    }
}

// ===== Runtime =====

/// Start server with given `Service`.
pub fn serve<L: Listener, S: Service<Request>>(io: L, service: S) -> Serve<L, S> {
    Serve { io, service: Arc::new(service) }
}

#[derive(Debug)]
pub struct Serve<L, S> {
    io: L,
    service: Arc<S>,
}

impl<L, S> Future for Serve<L, S>
where
    L: Listener,
    L::Stream: Send + 'static,
    S: Service<Request> + Send + Sync + 'static,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            match ready!(self.io.poll_accept(cx)) {
                Ok((io, _)) => {
                    let s = Arc::clone(&self.service);
                    tokio::spawn(Connection::new(io, s));
                },
                Err(err) => {
                    eprintln!("failed to serve peer: {err}");
                },
            }
        }
    }
}

// ===== Connection =====

const MAX_HEADERS: usize = 64;

struct Connection<IO, S> {
    io: BufReader<IO>,
    service: Arc<S>,
    phase: Phase,
}

enum Phase {
    Read,
    Parse,
}

impl<IO, S> Connection<IO, S>
where
    IO: AsyncIoRead + AsyncIoWrite,
    S: Service<Request>
{
    pub fn new(io: IO, service: Arc<S>) -> Self {
        Self { io: BufReader::new(io), service, phase: Phase::Read }
    }

    fn try_poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<io::Result<()>> {
        let me = unsafe { self.as_mut().get_unchecked_mut() };
        let mut phase = unsafe { Pin::new_unchecked(&mut me.phase) };

        loop {
            match phase.as_mut().get_mut() {
                Phase::Read => {
                    let read = ready!(me.io.poll_read_fill(cx)?);
                    if read == 0 {
                        return Poll::Ready(Ok(()));
                    }
                    phase.set(Phase::Parse);
                },
                Phase::Parse => {
                    let mut buf = me.io.chunk();
                    let Some(line) = parse_line(&mut buf)? else {
                        phase.set(Phase::Read);
                        continue;
                    };

                    let mut headers = [const { MaybeUninit::uninit() };MAX_HEADERS];

                    let Some(headers) = parse_headers_uninit(&mut buf, &mut headers)? else {
                        phase.set(Phase::Read);
                        continue;
                    };

                    println!("> {} {} {:?}", line.method, line.uri, line.version);

                    for header in headers {
                        println!("> {}: {:?}", header.name, str::from_utf8(header.value));
                    }

                    return Poll::Ready(Ok(()));
                },
                #[allow(unreachable_patterns, reason = "TODO more phase later")]
                _ => unreachable!(),
            }
        }
    }
}

impl<IO, S> Future for Connection<IO, S>
where
    IO: AsyncIoRead + AsyncIoWrite,
    S: Service<Request>
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        if let Err(err) = ready!(self.as_mut().try_poll(cx)) {
            eprintln!("failed to serve http: {err}");
        }

        Poll::Ready(())
    }
}

