use std::{
    future::Future,
    io,
    ops::{Deref, DerefMut},
    pin::{pin, Pin},
    task::{Context, Poll},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{body::Body, service::Service};

/// handle read and write buffer to `TcpStream`
///
/// this implements [`Service`] for working with socket reads
///
/// hold `TcpStream` and keep reading for new request
///
/// any io error will terminate connection
///
/// any inner service error or inner service response frame error also terminate connection
pub struct Connection<S> {
    inner: Option<S>,
}

impl<S> Connection<S> {
    pub fn new(service: S) -> Self {
        Self { inner: Some(service) }
    }
}

impl<S,B> Service<TcpStream> for Connection<S>
where
    S: Service<ConnectionBuffer,Response = B>,
    B: Body,
{
    type Response = ();
    type Error = ConnectionFutureError<S::Error,B::Error>;
    type Future = ConnectionFuture<S,B>;

    fn poll_ready(&mut self) -> Poll<Result<(), Self::Error>> {
        self.inner
            .as_mut()
            .expect("not yet called")
            .poll_ready()
            .map_err(ConnectionFutureError::Inner)
    }

    fn call(&mut self, request: TcpStream) -> Self::Future {
        ConnectionFuture {
            inner: self.inner.take().expect("only called once"),
            stream: request,
            buffer: Vec::with_capacity(1024),
            state: ConnectionFutureState::Read,
        }
    }
}

/// trait extension for creating [`StreamHandle`]
pub trait TcpStreamExt {
    fn handle(&mut self) -> StreamHandle;
}

impl TcpStreamExt for TcpStream {
    fn handle(&mut self) -> StreamHandle {
        let ptr = self as *mut TcpStream as usize;
        StreamHandle {
            ptr,
            stream: unsafe { &mut *{ ptr as *mut TcpStream } },
        }
    }
}

/// unsafe clonable [`TcpStream`] handle
pub struct StreamHandle {
    ptr: usize,
    stream: &'static mut TcpStream,
}

impl Clone for StreamHandle {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            stream: unsafe { &mut *{ self.ptr as *mut TcpStream } }
        }
    }
}

impl Deref for StreamHandle {
    type Target = TcpStream;

    fn deref(&self) -> &Self::Target {
        self.stream
    }
}

impl DerefMut for StreamHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.stream
    }
}

/// unsafe clonable [`Vec<u8>`] handle
pub struct BufferHandle {
    ptr: usize,
    buffer: &'static mut Vec<u8>,
}

impl BufferHandle {
    fn new(buffer: &mut Vec<u8>) -> Self {
        let ptr = buffer as *mut Vec<u8> as _;
        Self {
            ptr,
            buffer: unsafe { &mut *{ buffer as *mut Vec<u8> } },
        }
    }

    pub fn as_static(&self) -> &'static [u8] {
        unsafe { &*{ self.deref().deref() as *const [u8] } }
    }
}

impl Clone for BufferHandle {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            buffer: unsafe { &mut *{ self.ptr as *mut Vec<u8> } }
        }
    }
}

impl Deref for BufferHandle {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        self.buffer
    }
}

impl DerefMut for BufferHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
    }
}

/// a [`Service::Response`] for [`Connection`]
///
/// this struct contains stream handle and request buffer
pub struct ConnectionBuffer {
    buffer: BufferHandle,
    stream: StreamHandle,
}

impl ConnectionBuffer {
    pub fn buffer(&self) -> &[u8] {
        self.buffer.deref()
    }

    pub fn buffer_mut(&mut self) -> &mut Vec<u8> {
        self.buffer.deref_mut()
    }

    pub fn buffer_handle(&mut self) -> BufferHandle {
        self.buffer.clone()
    }

    pub fn stream(&self) -> &TcpStream {
        self.stream.deref()
    }

    pub fn stream_mut(&mut self) -> &mut TcpStream {
        self.stream.deref_mut()
    }

    pub fn stream_handle(&self) -> StreamHandle {
        self.stream.clone()
    }
}

/// socket event loop
///
/// this future will never resolve until socket closed or io error happens
// internally, it holds a `Vec<u8>` for request buffering that lives until
// the socket dropped
pub struct ConnectionFuture<S,B>
where
    S: Service<ConnectionBuffer>,
    B: Body,
{
    stream: TcpStream,
    buffer: Vec<u8>,
    inner: S,
    state: ConnectionFutureState<S::Future,B>,
}

impl<S, B> ConnectionFuture<S, B>
where
    S: Service<ConnectionBuffer>,
    B: Body,
{
    pub fn buffer_static(&self) -> &'static [u8] {
        unsafe { &*{ &*self.buffer as *const [u8] } }
    }

    pub fn buffer_static_mut(&mut self) -> &'static mut Vec<u8> {
        unsafe { &mut *{ &mut self.buffer as *mut Vec<u8> } }
    }

    pub fn handle(&mut self) -> StreamHandle {
        self.stream.handle()
    }
}

#[derive(Default)]
enum ConnectionFutureState<F,B>
where
    B: Body,
{
    /// reading request
    Read,
    /// calling inner handler `ready`
    HandleReady,
    /// calling inner handler `call`
    HandleCall { future: F },
    /// reading response frames
    ResBodyRead { body: B },
    /// writing response
    ResBodyWrite { body: B, frame: B::Output },
    #[default]
    End,
}

#[derive(thiserror::Error, Debug)]
pub enum ConnectionFutureError<E,B> {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Inner(E),
    #[error(transparent)]
    InnerBody(B),
}

impl<S,B> Unpin for ConnectionFuture<S,B>
where
    S: Service<ConnectionBuffer>,
    B: Body,
{}

impl<S,B> Future for ConnectionFuture<S,B>
where
    S: Service<ConnectionBuffer,Response = B>,
    B: Body
{
    type Output = Result<(), ConnectionFutureError<S::Error,B::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match std::mem::take(&mut self.state) {
                ConnectionFutureState::Read => {
                    let buffer = self.buffer_static_mut();
                    let pin = pin!(self.stream.read_buf(buffer));
                    match pin.poll(cx) {
                        Poll::Ready(Ok(0)) => return Poll::Ready(Ok(())),
                        Poll::Ready(Ok(_)) => {}
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(err.into())),
                        Poll::Pending => {
                            self.state = ConnectionFutureState::Read;
                            return Poll::Pending;
                        }
                    }

                    self.state = ConnectionFutureState::HandleReady;
                }
                ConnectionFutureState::HandleReady => {
                    match self.inner.poll_ready() {
                        Poll::Ready(Ok(_)) => {}
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(ConnectionFutureError::Inner(err))),
                        Poll::Pending => {
                            self.state = ConnectionFutureState::HandleReady;
                            return Poll::Pending;
                        }
                    }

                    let request = ConnectionBuffer {
                        buffer: BufferHandle::new(&mut self.buffer),
                        stream: self.handle()
                    };
                    let future = self.inner.call(request);
                    self.state = ConnectionFutureState::HandleCall { future };
                }
                ConnectionFutureState::HandleCall { mut future } => {
                    // SAFETY: because we never move future
                    let pin = unsafe { Pin::new_unchecked(&mut future) };
                    let body = match pin.poll(cx) {
                        Poll::Ready(Ok(ok)) => ok,
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(ConnectionFutureError::Inner(err))),
                        Poll::Pending => {
                            self.state = ConnectionFutureState::HandleCall { future };
                            return Poll::Pending;
                        }
                    };

                    self.state = ConnectionFutureState::ResBodyRead { body }
                }
                ConnectionFutureState::ResBodyRead { mut body } => {
                    if body.is_end_stream() {
                        self.buffer.clear();
                        self.state = ConnectionFutureState::Read;
                        continue;
                    }

                    let frame = match body.poll_frame() {
                        Poll::Ready(None) => todo!(),
                        Poll::Ready(Some(Ok(frame))) => frame,
                        Poll::Ready(Some(Err(err))) => {
                            return Poll::Ready(Err(ConnectionFutureError::InnerBody(err)))
                        },
                        Poll::Pending => {
                            self.state = ConnectionFutureState::ResBodyRead { body };
                            return Poll::Pending;
                        }
                    };

                    self.state = ConnectionFutureState::ResBodyWrite { body, frame };
                }
                ConnectionFutureState::ResBodyWrite { body, mut frame } => {
                    let pin = pin!(self.stream.write_all_buf(&mut frame));
                    match pin.poll(cx) {
                        Poll::Ready(Ok(_)) => {}
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(err.into())),
                        Poll::Pending => {
                            self.state = ConnectionFutureState::ResBodyWrite { body, frame };
                            return Poll::Pending;
                        }
                    }

                    self.state = ConnectionFutureState::ResBodyRead { body };
                }
                ConnectionFutureState::End => unreachable!("poll after complete"),
            }
        }
    }
}





