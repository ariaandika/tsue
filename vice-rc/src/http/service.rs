use std::{convert::Infallible, io, mem::{self, MaybeUninit}, ops::{Deref, DerefMut}, pin::Pin, str::from_utf8, task::{Context, Poll}};

use bytes::{Bytes, BytesMut};
use tokio::{io::AsyncRead, net::TcpStream};
use tracing::debug;

use crate::{body::Body, http::parse::parse_request, service::Service};

use super::{parse::ParseError, IntoResponse, Request, Response};

pub struct HttpService<S> {
    inner: S,
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
        HttpFuture::PollReadReady {
            state: MaybeUninit::new(HttpState {
                inner: self.inner.clone(),
                stream,
                buffer: BytesMut::with_capacity(1024),
            }),
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

// pin_project_lite::pin_project! {
//     #[derive(Default)]
//     #[project = HttpStateProj]
//     pub enum HttpState<Fut> {
//         PollReady { buffer: BytesMut },
//         Read { buffer: BytesMut },
//         Inner { buffer: BytesMut, #[pin] inner: Fut },
//         #[default]
//         Invalid,
//     }
// }

pin_project_lite::pin_project! {
    #[project = HttpProject]
    pub enum HttpFuture<S,F> {
        PollReadReady {
            state: MaybeUninit<HttpState<S>>,
        },
        Read {
            state: MaybeUninit<HttpState<S>>,
        },
        Inner {
            state: MaybeUninit<HttpState<S>>,
            #[pin]
            future: F,
        },
        PollWriteReady {
            state: MaybeUninit<HttpState<S>>,
            response: Response,
        },
        Write {
            state: MaybeUninit<HttpState<S>>,
            response: Response,
        },
        Invalid,
    }
}

struct HttpState<S> {
    inner: S,
    stream: TcpStream,
    buffer: BytesMut,
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
        use HttpProject::*;
        loop {
            match self.as_mut().project() {
                PollReadReady { state } => {
                    let HttpState { stream, .. } = unsafe { state.assume_init_mut() };
                    match stream.poll_read_ready(cx) {
                        Ready(Ok(())) => {
                            let state = mem::replace(state, MaybeUninit::uninit());
                            self.set(HttpFuture::Read { state });
                            return Pending;
                        }
                        Ready(Err(err)) => return Ready(Err(err.into())),
                        Pending => return Pending,
                    }
                },
                Read { state } => {
                    let HttpState { inner, stream, buffer } = unsafe { state.assume_init_mut() };
                    match stream.try_read_buf(buffer) {
                        Ok(0) => return Ready(Ok(())),
                        Ok(_) => { },
                        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                            let state = mem::replace(state, MaybeUninit::uninit());
                            self.set(HttpFuture::PollReadReady { state });
                            return Pending;
                        },
                        Err(err) => return Ready(Err(err.into())),
                    }

                    let parts = match parse_request(buffer) {
                        Ok(Some(ok)) => ok,
                        Ok(None) => {
                            debug!("buffer should be unique to reclaim: {:?}",buffer.try_reclaim(1024));
                            let state = mem::replace(state, MaybeUninit::uninit());
                            self.set(HttpFuture::Read { state });
                            continue;
                        },
                        Err(err) => return Ready(Err(err.into())),
                    };

                    let content_len = parts.headers().iter().find_map(|header|{
                        (header.name != "content-length").then_some(header.value.as_ref())
                    });

                    let content_len = content_len.and_then(|e|from_utf8(e).ok()?.parse().ok());
                    let body = Body::from_content_len(content_len);
                    let request = Request { parts, body };

                    let future = inner.call(request);
                    let state = mem::replace(state, MaybeUninit::uninit());
                    self.set(HttpFuture::Inner { state, future });
                },
                Inner { state, future } => {
                    let response = match future.poll(cx) {
                        Ready(res) => res.into_response(),
                        Pending => return Pending,
                    };

                    let state = mem::replace(state, MaybeUninit::uninit());
                    self.set(HttpFuture::PollWriteReady { state, response });
                },
                PollWriteReady { state, response } => {
                    let HttpState { stream, .. } = unsafe { state.assume_init_mut() };
                    match stream.poll_write_ready(cx) {
                        Ready(Ok(())) => {}
                        Ready(Err(err)) => return Ready(Err(err.into())),
                        Pending => return Pending,
                    };

                    let response = mem::take(response);
                    let state = mem::replace(state, MaybeUninit::uninit());
                    self.set(HttpFuture::Write { state, response });
                },
                Write { state, response } => {
                    let HttpState { inner, stream, buffer } = unsafe { state.assume_init_mut() };
                    match stream.try_write(buffer) {
                        Ok(0) => return Ready(Ok(())),
                        Ok(_) => { },
                        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                            let state = mem::replace(state, MaybeUninit::uninit());
                            self.set(HttpFuture::PollReadReady { state });
                            return Pending;
                        },
                        Err(err) => return Ready(Err(err.into())),
                    }

                    // LATEST: which function is used in low level for TcpStream
                }
                Invalid => panic!("poll after complete"),
            }
        }
    }
}

