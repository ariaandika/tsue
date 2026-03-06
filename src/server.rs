use std::io;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Poll, ready};
use tcio::io::{AsyncRead, AsyncWrite};

use crate::h1;
use crate::service::HttpService;

// ===== Server =====

#[derive(Debug)]
pub struct Server<S, L, D> {
    service: S,
    listener: L,
    _p: PhantomData<D>,
}

pub type Http1Server<S, L> = Server<S, L, Http1>;

impl<S, L, D> Server<S, L, D> {
    #[inline]
    pub fn new(service: S, listener: L) -> Self {
        Self {
            service,
            listener,
            _p: PhantomData,
        }
    }
}

impl<S, L, D> Future for Server<S, L, D>
where
    S: Send + Sync + Clone + 'static,
    L: Listener<Stream: AsyncRead + AsyncWrite>,
    D: Driver<S, L::Stream, Future: Future<Output: Send> + Send + 'static>,
{
    type Output = ();

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        let me = unsafe { self.get_unchecked_mut() };
        // SAFETY: `self` is pinned
        let mut listener = unsafe { Pin::new_unchecked(&mut me.listener) };

        loop {
            let (io, _) = match ready!(listener.as_mut().poll_accept(cx)) {
                Ok(ok) => ok,
                Err(err) => {
                    D::on_stream_error(err);
                    continue;
                }
            };

            tokio::spawn(D::call(me.service.clone(), io));
        }
    }
}

// ===== trait Driver =====

pub trait Driver<S, IO> {
    type Future;

    fn call(service: S, io: IO) -> Self::Future;

    #[inline]
    fn on_stream_error(err: io::Error) {
        eprintln!("{err}");
    }
}

// ===== Http1 Driver =====

#[derive(Debug)]
pub struct Http1;

impl<S, IO> Driver<S, IO> for Http1
where
    S: HttpService,
{
    type Future = h1::Connection<S, IO>;

    #[inline]
    fn call(service: S, io: IO) -> Self::Future {
        h1::Connection::new(service, io)
    }
}

// ===== Listener =====

pub trait Listener {
    type Stream: AsyncRead + AsyncWrite;

    type Addr;

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<(Self::Stream, Self::Addr)>>;
}

// ===== impl Listener =====

mod listener {
    use std::{io, net::SocketAddr, pin::Pin, task::Poll};
    use tokio::net::{TcpListener, TcpStream};
    use super::Listener;

    #[cfg(unix)]
    use tokio::net::{UnixListener, UnixStream};

    impl Listener for TcpListener {
        type Stream = TcpStream;

        type Addr = SocketAddr;

        #[inline]
        fn poll_accept(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context,
        ) -> Poll<io::Result<(Self::Stream, Self::Addr)>> {
            TcpListener::poll_accept(&self, cx)
        }
    }

    #[cfg(unix)]
    impl Listener for UnixListener {
        type Stream = UnixStream;

        type Addr = tokio::net::unix::SocketAddr;

        #[inline]
        fn poll_accept(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context,
        ) -> Poll<io::Result<(Self::Stream, Self::Addr)>> {
            UnixListener::poll_accept(&self, cx)
        }
    }
}
