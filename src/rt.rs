//! Runtime Server
use bytes::{BufMut, BytesMut};
use std::{
    io,
    mem::MaybeUninit,
    net::SocketAddr,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering::SeqCst},
    },
    task::{Poll, ready},
};
use tcio::{
    ByteStr,
    io::{AsyncIoRead, AsyncIoWrite},
    slice::{range_of, slice_of_bytes},
};
use tokio::net::{TcpListener, TcpStream};

#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

use crate::{
    body::{Body, BodyInner},
    headers::{HeaderMap, HeaderValue},
    http::Uri,
    request::{
        self, Request,
        parser::{RequestLine, parse_headers_range_uninit, parse_line},
    },
    response::Response,
    service::{HttpService, Service},
};

#[inline]
fn io_err<E: Into<Box<dyn std::error::Error + Send + Sync>>>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e)
}

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

// ===== Runtime =====

/// Start server with given `Service`.
#[inline]
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
    L: Listener<Stream = TcpStream>,
    S: HttpService + Send + Sync + 'static,
    S::Future: Send,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        loop {
            match ready!(self.io.poll_accept(cx)) {
                Ok((io, _)) => {
                    tokio::spawn(Connection::new(io, Arc::clone(&self.service)));
                },
                Err(err) => {
                    eprintln!("failed to serve peer: {err}");
                },
            }
        }
    }
}

// ===== Connection =====

struct Connection<S, F> {
    io_buffer: Arc<BodyInner>,
    service: Arc<S>,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    phase: Phase<F>,
}

enum Phase<F> {
    Read,
    Parse,
    Service(F, Context),
    Response(Context),
    Write(Context),
    WriteBody(Context),
    Cleanup,
}

impl<S: HttpService> Connection<S, S::Future> {
    fn new(io: TcpStream, service: Arc<S>) -> Self {
        Self {
            io_buffer: Arc::new(BodyInner::new(io, AtomicU64::new(0))),
            service,
            read_buffer: BytesMut::with_capacity(1024),
            write_buffer: BytesMut::with_capacity(1024),
            phase: Phase::Read,
        }
    }

    fn try_poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<io::Result<()>> {
        const MAX_HEADERS: usize = 64;

        // SAFETY: self is pinned
        // no `Drop`, nor manual `Unpin` implementation.
        let me = unsafe { self.as_mut().get_unchecked_mut() };

        loop {
            match &mut me.phase {
                Phase::Read => {
                    let read = ready!(me.io_buffer.io.poll_read_buf(&mut me.read_buffer, cx)?);
                    if read == 0 {
                        return Poll::Ready(Ok(()));
                    }
                    me.phase = Phase::Parse;
                },
                Phase::Parse => {
                    let mut buf = me.read_buffer.as_ref();
                    let start_ptr = buf.as_ptr() as usize;
                    let Some(RequestLine { method, uri, version }) = parse_line(&mut buf)? else {
                        me.phase = Phase::Read;
                        continue;
                    };

                    let uri_range = range_of(uri.as_bytes());

                    let mut headers = [const { MaybeUninit::uninit() };MAX_HEADERS];

                    let Some(headers_range) = parse_headers_range_uninit(&mut buf, &mut headers)? else {
                        me.phase = Phase::Read;
                        continue;
                    };

                    let head_len = buf.as_ptr() as usize - start_ptr;

                    // ===== parse complete =====

                    let remain_buf = me.read_buffer.split().freeze();

                    // SAFETY: uri_range come from `uri` which is a `str`
                    let uri = unsafe { ByteStr::from_utf8_unchecked(slice_of_bytes(uri_range, &remain_buf)) };
                    let uri = Uri::try_from_shared(uri).map_err(io_err)?;

                    let mut headers = HeaderMap::with_capacity(headers_range.len());
                    let mut content_len = None::<u64>;

                    for range in headers_range {
                        let name = range.resolve_name(&remain_buf);
                        let value = range.resolve_value(&remain_buf);

                        if name.eq_ignore_ascii_case("content-length") {
                            content_len = str::from_utf8(&value).ok().and_then(|e|e.parse().ok());
                        }

                        if let Ok(value) = HeaderValue::try_from_slice(value) {
                            headers.insert(name, value);
                        }
                    }

                    me.io_buffer.remaining.store(content_len.unwrap_or_default(), SeqCst);

                    let parts = request::Parts {
                        method,
                        uri,
                        version,
                        headers,
                        extensions: <_>::default(),
                    };

                    let remain_body = remain_buf.slice(head_len..);
                    let remain_len = remain_body.len().try_into().unwrap_or(u64::MAX);
                    let body = Body::from_io(Arc::clone(&me.io_buffer), remain_body.clone());
                    let request = Request::from_parts(parts, body);

                    let fut = me.service.call(request);

                    me.phase = Phase::Service(fut, Context::new(remain_len));
                },
                Phase::Service(f, ctx) => {
                    // SAFETY: self is pinned
                    let f = unsafe { Pin::new_unchecked(f) };
                    let response = match ready!(f.poll(cx)) {
                        Ok(ok) => ok,
                        Err(err) => return Poll::Ready(Err(io_err(err))),
                    };

                    ctx.set_response(response);

                    me.phase = Phase::Response(ctx.take());
                },
                Phase::Response(ctx) => {
                    let res = ctx.response.as_ref().unwrap();

                    // Drain unread body if any
                    if me.io_buffer.has_remaining() {
                        me.io_buffer.remaining.fetch_sub(ctx.remain, SeqCst);
                    }
                    while me.io_buffer.has_remaining() {
                        ready!(me.io_buffer.poll_read_buf(&mut me.write_buffer, cx)?);
                        me.write_buffer.clear();
                    }

                    // write response headline to intermediate buffer
                    // maybe custom statefull write vectored for Response ?

                    let parts = res.parts();
                    let body = res.body();

                    me.write_buffer.put_slice(parts.version.as_str().as_bytes());
                    me.write_buffer.put_slice(b" ");
                    me.write_buffer.put_slice(parts.status.as_str().as_bytes());
                    me.write_buffer.put_slice(b"\r\nDate: ");
                    me.write_buffer.put_slice(&crate::http::httpdate_now());
                    me.write_buffer.put_slice(b"\r\nContent-Length: ");
                    me.write_buffer.put_slice(itoa::Buffer::new().format(body.exact_len()).as_bytes());
                    me.write_buffer.put_slice(b"\r\n");
                    for (name, value) in &parts.headers {
                        me.write_buffer.put_slice(name.as_str().as_bytes());
                        me.write_buffer.put_slice(b": ");
                        me.write_buffer.put_slice(value.as_bytes());
                        me.write_buffer.put_slice(b"\r\n");
                    }
                    me.write_buffer.put_slice(b"\r\n");

                    me.phase = Phase::Write(ctx.take());
                },
                Phase::Write(response) => {
                    ready!(me.io_buffer.io.poll_write_all_buf(&mut me.write_buffer, cx)?);
                    me.phase = Phase::WriteBody(response.take());
                },
                Phase::WriteBody(ctx) => {
                    let response = ctx.response.as_mut().unwrap();
                    ready!(me.io_buffer.io.poll_write_all_buf(response.body_mut().bytes_mut(), cx)?);
                    me.phase = Phase::Cleanup;
                },
                Phase::Cleanup => {
                    me.read_buffer.clear();
                    me.write_buffer.clear();
                    me.read_buffer.reserve(1024);
                    me.write_buffer.reserve(1024);
                    me.phase = Phase::Read;
                },
            }
        }
    }
}

impl<S: HttpService> Future for Connection<S, S::Future> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        if let Err(err) = ready!(self.as_mut().try_poll(cx)) {
            eprintln!("failed to serve http: {err}");
        }

        Poll::Ready(())
    }
}

// ===== Context =====

struct Context {
    remain: u64,
    response: Option<Response>,
}

impl Context {
    fn new(remain: u64) -> Self {
        Self { remain, response: None }
    }

    fn set_response(&mut self, response: Response) {
        self.response = Some(response);
    }

    fn take(&mut self) -> Self {
        Self {
            remain: self.remain,
            response: self.response.take(),
        }
    }
}


