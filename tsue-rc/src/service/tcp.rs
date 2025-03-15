use super::HttpService;
use crate::{
    body::Body,
    request::{self, Request},
    response::{self, IntoResponse},
    service::Service,
    task::{StreamFuture, StreamHandle},
};
use bytes::BytesMut;
use log::{debug, error, trace};
use std::{
    mem,
    pin::Pin,
    str::from_utf8,
    task::{Context, Poll},
};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct TcpService<S> {
    inner: S,
}

impl<S> TcpService<S> {
    pub fn new(inner: S) -> TcpService<S> {
        TcpService { inner }
    }
}

impl<S> Service<TcpStream> for TcpService<S>
where
    S: HttpService + Clone
{
    type Response = ();
    // error will only be ignored in top level service
    type Error = ();
    type Future = TcpFuture<S,S::Future>;

    fn call(&self, stream: TcpStream) -> Self::Future {
        trace!("connection open");
        TcpFuture {
            inner: self.inner.clone(),
            buffer: BytesMut::with_capacity(1024),
            res_buffer: BytesMut::with_capacity(1024),
            stream: crate::task::spawn(stream),
            state: TcpState::Init,
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
    #[project = TcpStateProject]
    enum TcpState<Fut> {
        Init,
        Read { #[pin] rx: StreamFuture<(usize,BytesMut)> },
        Parse,
        Inner { #[pin] future: Fut },
        Write { #[pin] rx: StreamFuture<()> },
        Cleanup,
        #[default]
        Invalid,
    }
}

pin_project_lite::pin_project! {
    #[project = TcpProject]
    pub struct TcpFuture<S,F> {
        inner: S,
        buffer: BytesMut,
        res_buffer: BytesMut,
        stream: StreamHandle,
        #[pin]
        state: TcpState<F>,
    }
}

impl<S> Future for TcpFuture<S,S::Future>
where
    S: Service<Request>,
    S::Response: IntoResponse,
    S::Error: IntoResponse,
{
    type Output = Result<(),()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use Poll::*;
        use TcpStateProject::*;

        let TcpProject {
            inner,
            buffer,
            res_buffer,
            stream,
            mut state,
        } = self.as_mut().project();

        loop {
            match state.as_mut().project() {
                Init => {
                    let rx = stream.read(mem::take(buffer));
                    state.set(TcpState::Read { rx });
                }
                Read { rx } => {
                    let (read,rx) = ready!(rx.poll(cx));
                    *buffer = rx;
                    if read == 0 {
                        trace!("connection closed");
                        return Ready(Ok(()));
                    }
                    state.set(TcpState::Parse);
                }
                Parse => {
                    let parts = match unwrap!(request::parse(buffer)) {
                        Some(ok) => ok,
                        None => {
                            debug!("buffer should be unique to reclaim: {:?}",buffer.try_reclaim(1024));
                            state.set(TcpState::Init);
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

                    // debug!("bytes body: {buffer:?}");

                    let request = Request::from_parts(parts,body);
                    let future = inner.call(request);
                    state.set(TcpState::Inner { future });
                }
                Inner { future } => {
                    let mut response = match future.poll(cx) {
                        Ready(res) => res.into_response(),
                        Pending => return Pending,
                    };

                    response::check(&mut response);
                    let (parts,body) = response.into_parts();
                    response::write(&parts, res_buffer);
                    let rx = stream.write(res_buffer.split().freeze(), body);
                    state.set(TcpState::Write { rx });
                }
                Write { rx } => {
                    // wait write complete
                    ready!(rx.poll(cx));
                    state.set(TcpState::Cleanup);
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

                    state.set(TcpState::Init);
                }
                Invalid => panic!("poll after complete"),
            }
        }
    }
}

