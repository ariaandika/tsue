use std::{
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Poll, ready},
};

use tcio::io::{AsyncIoRead, AsyncIoWrite};

pub use driver::Driver;
pub use listener::Listener;

pub type Http1Server<L, S> = Server<L, S, driver::Http1>;

#[derive(Debug)]
pub struct Server<L, S, D> {
    listener: L,
    service: Arc<S>,
    _p: PhantomData<D>,
}

impl<L, S, D> Server<L, S, D> {
    pub fn new(listener: L, service: S) -> Self {
        Self {
            listener,
            service: Arc::new(service),
            _p: PhantomData,
        }
    }
}

impl<L, S, D> Future for Server<L, S, D>
where
    L: Listener<Stream: AsyncIoRead + AsyncIoWrite>,
    D: Driver<L::Stream, S, Future: Future<Output: Send> + Send + 'static>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        loop {
            let (io, _) = match ready!(self.listener.poll_accept(cx)) {
                Ok(ok) => ok,
                Err(err) => {
                    eprintln!("{err}");
                    continue;
                }
            };

            tokio::spawn(D::call(io, Arc::clone(&self.service)));
        }
    }
}

pub mod driver {
    use std::sync::Arc;

    use crate::{h1::connection::Connection, service::HttpService};

    pub trait Driver<IO, S> {
        type Future;

        fn call(io: IO, service: Arc<S>) -> Self::Future;
    }

    #[derive(Debug)]
    pub struct Http1;

    impl<IO, S> Driver<IO, S> for Http1
    where
        S: HttpService,
    {
        type Future = Connection<IO, S, S::Future>;

        fn call(io: IO, service: Arc<S>) -> Self::Future {
            Connection::new(io, service)
        }
    }
}

mod listener {
    use std::{io, net::SocketAddr, task::Poll};
    use tcio::io::{AsyncIoRead, AsyncIoWrite};
    use tokio::net::{TcpListener, TcpStream};

    #[cfg(unix)]
    use tokio::net::{UnixListener, UnixStream};

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
