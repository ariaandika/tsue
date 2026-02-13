use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Poll, ready};
use tcio::io::{AsyncRead, AsyncWrite};

pub use driver::Driver;
pub use listener::Listener;

pub type Http1Server<L, S> = Server<L, S, driver::Http1>;

#[derive(Debug)]
pub struct Server<L, S, D> {
    listener: L,
    service: S,
    _p: PhantomData<D>,
}

impl<L, S, D> Server<L, S, D> {
    #[inline]
    pub fn new(listener: L, service: S) -> Self {
        Self {
            listener,
            service,
            _p: PhantomData,
        }
    }
}

impl<L, S, D> Future for Server<L, S, D>
where
    L: Listener<Stream: AsyncRead + AsyncWrite>,
    S: Send + Sync + Clone + 'static,
    D: Driver<L::Stream, S, Future: Future<Output: Send> + Send + 'static>,
{
    type Output = ();

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        loop {
            let (io, _) = match ready!(self.listener.poll_accept(cx)) {
                Ok(ok) => ok,
                Err(err) => {
                    eprintln!("{err}");
                    continue;
                }
            };

            tokio::spawn(D::call(io, self.service.clone()));
        }
    }
}

pub mod driver {
    use crate::{h1::connection::Connection, service::HttpService};

    pub trait Driver<IO, S> {
        type Future;

        fn call(io: IO, service: S) -> Self::Future;
    }

    #[derive(Debug)]
    pub struct Http1;

    impl<IO, S, B> Driver<IO, S> for Http1
    where
        S: HttpService<ResBody = B>,
        B: crate::body::Body,
        B::Error: std::error::Error + Send + Sync + 'static,
    {
        type Future = Connection<IO, S, B, B::Data, S::Future>;

        #[inline]
        fn call(io: IO, service: S) -> Self::Future {
            Connection::new(io, service)
        }
    }
}

mod listener {
    use std::{io, net::SocketAddr, task::Poll};
    use tcio::io::{AsyncRead, AsyncWrite};
    use tokio::net::{TcpListener, TcpStream};

    #[cfg(unix)]
    use tokio::net::{UnixListener, UnixStream};

    pub trait Listener {
        type Stream: AsyncRead + AsyncWrite;

        type Addr;

        fn poll_accept(
            &self,
            cx: &mut std::task::Context,
        ) -> Poll<io::Result<(Self::Stream, Self::Addr)>>;
    }

    impl Listener for TcpListener {
        type Stream = TcpStream;

        type Addr = SocketAddr;

        #[inline]
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

        #[inline]
        fn poll_accept(
            &self,
            cx: &mut std::task::Context,
        ) -> Poll<io::Result<(Self::Stream, Self::Addr)>> {
            UnixListener::poll_accept(self, cx)
        }
    }
}
