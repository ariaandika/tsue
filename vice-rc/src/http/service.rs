use super::{request::ParseError, IntoResponse, Request};
use crate::{
    body::Body,
    http::request,
    service::Service,
    stream::{self, StreamFuture, StreamHandle},
};
use bytes::BytesMut;
use log::{debug, error, trace};
use std::{
    io::{self},
    mem,
    pin::Pin,
    str::from_utf8,
    task::{Context, Poll},
};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct HttpService<S> {
    inner: S,
}

impl<S> HttpService<S> {
    pub fn new(inner: S) -> HttpService<S> {
        HttpService { inner }
    }
}

impl<S> Service<TcpStream> for HttpService<S>
where
    S: Service<Request> + Clone,
    S::Response: IntoResponse,
    S::Error: IntoResponse,
{
    type Response = ();
    // error will only be ignored in top level service
    type Error = ();
    type Future = HttpFuture<S,S::Future>;

    fn call(&self, stream: TcpStream) -> Self::Future {
        trace!("connection open");
        HttpFuture {
            inner: self.inner.clone(),
            buffer: BytesMut::with_capacity(1024),
            res_buffer: BytesMut::with_capacity(1024),
            stream: stream::new_task(stream),
            state: HttpState::Init,
        }
    }
}

macro_rules! unwrap {
    ($body:expr) => {
        match $body {
            Ok(ok) => ok,
            Err(err) => {
                error!("{err}");
                return Poll::Ready(Ok(()))
            },
        }
    };
}

macro_rules! ready {
    ($body:expr) => {
        match $body {
            Ready(result) => unwrap!(result),
            Pending => return Pending,
        }
    };
}

pin_project_lite::pin_project! {
    #[derive(Default)]
    #[project = HttpStateProject]
    enum HttpState<Fut> {
        Init,
        Read { #[pin] rx: StreamFuture<io::Result<(usize,BytesMut)>> },
        Parse,
        Inner { #[pin] future: Fut },
        Write { #[pin] rx: StreamFuture<io::Result<()>> },
        Cleanup,
        #[default]
        Invalid,
    }
}

pin_project_lite::pin_project! {
    #[project = HttpProject]
    pub struct HttpFuture<S,F> {
        inner: S,
        buffer: BytesMut,
        res_buffer: BytesMut,
        stream: StreamHandle,
        #[pin]
        state: HttpState<F>,
    }
}

impl<S> Future for HttpFuture<S,S::Future>
where
    S: Service<Request>,
    S::Response: IntoResponse,
    S::Error: IntoResponse,
{
    type Output = Result<(),()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use Poll::*;
        use HttpStateProject::*;

        let HttpProject {
            inner,
            buffer,
            mut res_buffer,
            stream,
            mut state,
        } = self.as_mut().project();

        loop {
            match state.as_mut().project() {
                Init => {
                    let rx = stream.read(mem::take(buffer));
                    state.set(HttpState::Read { rx });
                }
                Read { rx } => {
                    let (read,rx) = ready!(rx.poll(cx));
                    *buffer = rx;
                    if read == 0 {
                        trace!("connection closed");
                        return Ready(Ok(()));
                    }
                    state.set(HttpState::Parse);
                }
                Parse => {
                    let parts = match unwrap!(request::parse(buffer)) {
                        Some(ok) => ok,
                        None => {
                            debug!("buffer should be unique to reclaim: {:?}",buffer.try_reclaim(1024));
                            state.set(HttpState::Init);
                            continue;
                        },
                    };

                    let content_len = parts
                        .headers()
                        .iter()
                        .find_map(|header| (header.name == "content-length").then_some(header.value.as_ref()))
                        .and_then(|e| from_utf8(e).ok()?.parse().ok());
                    let body = match content_len {
                        Some(len) => Body::new(len, buffer.split(), stream.clone()),
                        None => Body::empty(),
                    };

                    let request = Request::from_parts(parts,body);
                    let future = inner.call(request);
                    state.set(HttpState::Inner { future });
                }
                Inner { future } => {
                    let mut response = match future.poll(cx) {
                        Ready(res) => res.into_response(),
                        Pending => return Pending,
                    };

                    response.check();
                    let (parts,body) = response.into_parts();
                    parts.write(&mut res_buffer);
                    let rx = stream.write(res_buffer.split().freeze(), body);
                    state.set(HttpState::Write { rx });
                }
                Write { rx } => {
                    // wait write complete
                    ready!(rx.poll(cx));
                    state.set(HttpState::Cleanup);
                }
                Cleanup => {
                    // this state will make sure all shared buffer is dropped
                    res_buffer.clear();
                    buffer.clear();

                    if !buffer.try_reclaim(1024) {
                        debug!("failed to reclaim buffer");
                    }

                    if !res_buffer.try_reclaim(1024) {
                        debug!("failed to reclaim res_buffer");
                    }

                    state.set(HttpState::Init);
                }
                Invalid => panic!("poll after complete"),
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum HttpError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("parse error: {0}")]
    ParseError(#[from] ParseError),
}

/*
pub struct StreamActor(ActorHandle<Message>);

pub enum Message {
    Read {
        bytes: BytesMut,
        send: oneshot::Sender<io::Result<(usize,BytesMut)>>,
    },
    Write {
        head: Bytes,
        body: ResBody,
        send: oneshot::Sender<io::Result<()>>,
    },
}

impl StreamActor {
    fn new(stream: TcpStream) -> StreamActor {
        Self(Actor::new(move|recv|Self::task(stream, recv)))
    }

    fn read(&self, bytes: BytesMut) -> oneshot::Receiver<io::Result<(usize, BytesMut)>> {
        let (tx, rx) = oneshot::channel();
        match self.0.try_send(Message::Read { bytes, send: tx }) {
            Ok(()) => rx,
            Err(err) => {
                let msg = format!("actor should never full or closed: {err}");
                let tx = match err {
                    TrySendError::Full(Message::Read { send, .. }) |
                    TrySendError::Closed(Message::Read { send, .. }) => send,
                    _ => unreachable!(),
                };

                tx.send(Err(io::Error::new(io::ErrorKind::Other, msg)));

                rx
            }
        }
    }

    fn write(&self, head: Bytes, body: ResBody) -> io::Result<oneshot::Receiver<io::Result<()>>> {
        let (tx, rx) = oneshot::channel();
        match self.0.try_send(Message::Write { head, body, send: tx, }) {
            Ok(()) => Ok(rx),
            Err(err) => Err(into_io_error(err)),
        }
    }

    async fn task(mut stream: TcpStream, mut recv: mpsc::Receiver<Message>) {
        while let Some(message) = recv.recv().await {
            match message {
                Message::Read { mut bytes, send } => {
                    let _ = match stream.read_buf(&mut bytes).await {
                        Ok(ok) => send.send(Ok((ok,bytes))),
                        Err(err) => send.send(Err(err)),
                    };
                }
                Message::Write { head, body, send } => {
                    let _ = match stream.write_all_buf(&mut Bytes::chain(head, body.as_ref())).await {
                        Ok(_) => send.send(Ok(())),
                        Err(err) => send.send(Err(err)),
                    };
                }
            }
        }
    }
}

fn into_io_error<E: std::error::Error>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("actor should never full or closed: {err}"))
}
*/
