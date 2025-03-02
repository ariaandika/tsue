use super::{request::ParseError, IntoResponse, Request};
use crate::{body::{Body, ResBody}, http::request, service::Service};
use bytes::BytesMut;
use log::{debug, trace};
use std::{
    io::{self, IoSlice},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};

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
    type Error = HttpError;
    type Future = HttpFuture<S,S::Future>;

    fn call(&self, stream: TcpStream) -> Self::Future {
        trace!("connection open");
        HttpFuture {
            inner: self.inner.clone(),
            buffer: BytesMut::with_capacity(1024),
            res_buffer: BytesMut::with_capacity(1024),
            stream,
            state: HttpState::Read,
        }
    }
}

pin_project_lite::pin_project! {
    #[derive(Default)]
    #[project = HttpStateProject]
    enum HttpState<Fut> {
        Read,
        Parse,
        Inner { #[pin] future: Fut },
        Write { body: ResBody },
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
        #[pin]
        stream: TcpStream,
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
    type Output = Result<(),HttpError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use Poll::*;
        use HttpStateProject::*;

        let HttpProject { inner, buffer, mut res_buffer, mut stream, mut state } = self.as_mut().project();

        loop {
            match state.as_mut().project() {
                Read => {
                    let mut readbuf = ReadBuf::uninit(buffer.spare_capacity_mut());
                    match AsyncRead::poll_read(stream.as_mut(), cx, &mut readbuf) {
                        Ready(Ok(())) => {}
                        Ready(Err(err)) => return Ready(Err(err.into())),
                        Pending => return Pending,
                    }

                    let len = readbuf.filled().len();

                    if len == 0 {
                        trace!("connection closed");
                        return Ready(Ok(()));
                    }

                    // SAFETY: gloomer
                    unsafe { BytesMut::set_len(buffer, len) };

                    state.set(HttpState::Parse);
                }
                Parse => {
                    let parts = match request::parse(buffer) {
                        Ok(Some(ok)) => ok,
                        Ok(None) => {
                            debug!("buffer should be unique to reclaim: {:?}",buffer.try_reclaim(1024));
                            state.set(HttpState::Read);
                            continue;
                        },
                        Err(err) => return Ready(Err(err.into())),
                    };

                    // let content_len = parts.headers().iter().find_map(|header|{
                    //     (header.name != "content-length").then_some(header.value.as_ref())
                    // });
                    // let content_len = content_len.and_then(|e|from_utf8(e).ok()?.parse().ok());
                    // let body = match content_len {
                    //     Some(len) => Body::new(len),
                    //     None => Body::empty(),
                    // };

                    let body = Body::empty();
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
                    state.set(HttpState::Write { body });
                }
                Write { body } => {
                    let vectored = [IoSlice::new(&res_buffer),IoSlice::new(body.as_ref())];
                    match AsyncWrite::poll_write_vectored(stream.as_mut(), cx, &vectored) {
                        Ready(Ok(_)) => {}
                        Ready(Err(err)) => return Ready(Err(err.into())),
                        Pending => return Pending,
                    }

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

                    state.set(HttpState::Read);
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

