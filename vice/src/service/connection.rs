use std::{future::Future, io, pin::{pin, Pin}, task::{Context, Poll}};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

use crate::service::Service;

/// handle read and write buffer to `TcpStream`
///
/// specifically, handle `Service<(TcpStream,Vec<u8>), Response = (TcpStream,Vec<u8>)>` as `Service<TcpStream>`
///
/// hold `TcpStream` and keep `read_buf` for new request
///
/// any `read_buf` error will terminate connection
///
/// any inner service error also terminate connection
pub struct Connection<S> {
    inner: Option<S>,
}

impl<S> Connection<S> {
    pub fn new(service: S) -> Self {
        Self { inner: Some(service) }
    }
}

impl<S> Service<TcpStream> for Connection<S>
where
    S: Service<(TcpStream,Vec<u8>),Response = (TcpStream,Vec<u8>)>,
{
    type Response = TcpStream;
    type Error = ConnectionFutureError<S::Error>;
    type Future = ConnectionFuture<S>;

    fn call(&mut self, request: TcpStream) -> Self::Future {
        ConnectionFuture {
            inner: self.inner.take().expect("only called once"),
            state: ConnectionFutureState::Read {
                stream: request,
                buf: Vec::with_capacity(1024),
            }
        }
    }
}

pub struct ConnectionFuture<S>
where
    S: Service<(TcpStream,Vec<u8>)>,
{
    inner: S,
    state: ConnectionFutureState<S::Future>,
}

#[derive(Default)]
enum ConnectionFutureState<F> {
    /// reading request
    Read { stream: TcpStream, buf: Vec<u8> },
    /// calling inner handler `ready`
    HandleReady { stream: TcpStream, buf: Vec<u8> },
    /// calling inner handler `call`
    HandleCall { future: F },
    /// writing response
    Write { stream: TcpStream, buf: Vec<u8> },
    #[default]
    End,
}

#[derive(thiserror::Error, Debug)]
pub enum ConnectionFutureError<E> {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Inner(E),
}

impl<S> Unpin for ConnectionFuture<S>
where
    S: Service<(TcpStream,Vec<u8>)>,
{}

impl<S> Future for ConnectionFuture<S>
where
    S: Service<(TcpStream,Vec<u8>),Response = (TcpStream,Vec<u8>)>,
{
    type Output = Result<TcpStream, ConnectionFutureError<S::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match std::mem::take(&mut self.state) {
                ConnectionFutureState::Read { mut stream, mut buf } => {
                    let pin = pin!(stream.read_buf(&mut buf));
                    match pin.poll(cx) {
                        Poll::Ready(Ok(0)) => return Poll::Ready(Ok(stream)),
                        Poll::Ready(Ok(_)) => {}
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(err.into())),
                        Poll::Pending => {
                            self.state = ConnectionFutureState::Read { stream, buf };
                            return Poll::Pending;
                        }
                    }

                    self.state = ConnectionFutureState::HandleReady { stream, buf };
                }
                ConnectionFutureState::HandleReady { stream, buf } => {
                    match self.inner.poll_ready() {
                        Poll::Ready(Ok(_)) => {}
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(ConnectionFutureError::Inner(err))),
                        Poll::Pending => {
                            self.state = ConnectionFutureState::HandleReady { stream, buf };
                            return Poll::Pending;
                        }
                    }

                    let future = self.inner.call((stream,buf));
                    self.state = ConnectionFutureState::HandleCall { future };
                }
                ConnectionFutureState::HandleCall { mut future } => {
                    // SAFETY: because we never move future
                    let pin = unsafe { Pin::new_unchecked(&mut future) };
                    let (stream, buf) = match pin.poll(cx) {
                        Poll::Ready(Ok(ok)) => ok,
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(ConnectionFutureError::Inner(err))),
                        Poll::Pending => {
                            self.state = ConnectionFutureState::HandleCall { future };
                            return Poll::Pending;
                        }
                    };

                    self.state = ConnectionFutureState::Write { stream, buf }
                }
                ConnectionFutureState::Write { mut stream, mut buf } => {
                    let pin = pin!(stream.write_all(&buf));
                    match pin.poll(cx) {
                        Poll::Ready(Ok(_)) => {}
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(err.into())),
                        Poll::Pending => {
                            self.state = ConnectionFutureState::Write { stream, buf };
                            return Poll::Pending;
                        }
                    }

                    buf.clear();
                    self.state = ConnectionFutureState::Read { stream, buf };
                }
                ConnectionFutureState::End => unreachable!("poll after complete"),
            }
        }
    }
}





