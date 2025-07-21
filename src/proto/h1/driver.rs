use futures_core::ready;
use std::{io, mem::MaybeUninit, pin::Pin, sync::Arc, task::Poll};
use tcio::{
    ByteStr,
    io::{AsyncIoRead, AsyncIoWrite},
    slice::{range_of, slice_of_bytes},
};

use crate::{
    body::Body,
    headers::{HeaderMap, HeaderValue},
    http::{Extensions, Uri, httpdate_now},
    proto::h1::io::BodyWrite,
    request::{Parts, Request},
    response::Response,
    service::HttpService,
};

use super::{io::IoBuffer, parser};

macro_rules! retry_read {
    ($e:expr) => {
        match $e {
            Ok(Some(ok)) => ok,
            Ok(None) => continue,
            Err(err) => return Poll::Ready(Err(err)),
        }
    };
}

pub struct Connection<IO, S, F> {
    io: IoBuffer<IO>,
    header_map: Option<HeaderMap>,
    service: Arc<S>,

    phase: Phase<F>,
}

enum Phase<F> {
    Read,
    Service(F),
    Drain(Option<Response>),
    Flush(BodyWrite),
    Cleanup,
}

const MAX_HEADERS: usize = 64;

impl<IO, S> Connection<IO, S, S::Future>
where
    S: HttpService,
{
    pub fn new(io: IO, service: Arc<S>) -> Self {
        Self {
            header_map: None,
            io: IoBuffer::new(io),
            service,
            phase: Phase::Read,
        }
    }
}

type ProjectResult<'a, IO, F, S> = (
    &'a mut IoBuffer<IO>,
    &'a mut Option<HeaderMap>,
    Pin<&'a mut Phase<F>>,
    &'a mut Arc<S>,
);

impl<IO, S, F> Connection<IO, S, F> {
    fn project(self: Pin<&mut Self>) -> ProjectResult<'_, IO, F, S> {
        // SAFETY: self is pinned
        // and no custom Drop and Unpin
        unsafe {
            let me = self.get_unchecked_mut();
            (
                &mut me.io,
                &mut me.header_map,
                Pin::new_unchecked(&mut me.phase),
                &mut me.service,
            )
        }
    }
}

impl<IO, S> Connection<IO, S, S::Future>
where
    IO: AsyncIoRead + AsyncIoWrite,
    S: HttpService<Error: Into<Box<dyn std::error::Error + Send + Sync>>>,
{
    pub fn try_poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<io::Result<()>> {
        let (io, header_map, mut phase, service) = self.project();

        loop {
            match phase.as_mut().project() {
                Project::Read => {
                    let read = ready!(io.poll_read(cx))?;
                    if read == 0 {
                        return Poll::Ready(Ok(()));
                    }

                    let mut chunk = io.read_buffer();
                    let offset = chunk.as_ptr() as usize;

                    let parser::RequestLineRef {
                        method,
                        uri,
                        version,
                    } = retry_read!(parser::parse_line(&mut chunk));
                    let uri_range = range_of(uri.as_bytes());

                    let mut headers = [const { MaybeUninit::uninit() }; MAX_HEADERS];
                    let headers_ref =
                        retry_read!(parser::parse_headers_range_uninit(&mut chunk, &mut headers));

                    // parse complete

                    let read = chunk.as_ptr() as usize - offset;
                    let buf = io.read_buffer_mut().split_to(read).freeze();

                    // SAFETY: `uri_range` is from `uri` which is str, and `buf` is not mutated
                    let uri_str =
                        unsafe { ByteStr::from_utf8_unchecked(slice_of_bytes(uri_range, &buf)) };
                    let uri = Uri::try_from_shared(uri_str).map_err(io_err)?;

                    let mut headers = match header_map.take() {
                        Some(mut map) => {
                            map.reserve(headers_ref.len());
                            map
                        }
                        None => HeaderMap::with_capacity(headers_ref.len()),
                    };

                    let mut content_len = None;

                    for header in headers_ref {
                        // SAFETY: `buf` is not mutated
                        let name = unsafe { header.resolve_name_unchecked(&buf) };
                        let value = header.resolve_value(&buf);

                        if name.eq_ignore_ascii_case("content-length") {
                            content_len = Some(
                                str::from_utf8(&value)
                                    .map_err(io_err)
                                    .and_then(|e| e.parse().map_err(io_err))?,
                            );
                        }

                        if let Ok(value) = HeaderValue::try_from_slice(value) {
                            headers.insert(name, value);
                        }
                    }

                    let content_len: u64 = content_len.unwrap_or(0);
                    let remain = io.read_buffer_mut().split();

                    // at this point, buffer is empty, so reserve will not need to copy any data if
                    // allocation required
                    io.read_buffer_mut().reserve(content_len as _);

                    // `IoBuffer` remaining is only calculated excluding the already read body
                    let Some(remaining) = content_len.checked_sub(remain.len() as _) else {
                        return Poll::Ready(Err(io_err("content-length is less than body")));
                    };
                    io.set_remaining(remaining);

                    let parts = Parts {
                        method,
                        uri,
                        version,
                        headers,
                        extensions: Extensions::new(),
                    };

                    let body = Body::from_handle(io.get_handle(), content_len, remain);

                    let request = Request::from_parts(parts, body);

                    let f = service.call(request);
                    phase.set(Phase::Service(f));
                }
                Project::Service(f) => {
                    let Poll::Ready(res) = f.poll(cx).map_err(io_err)? else {
                        if ready!(io.poll_io_wants(cx))? {
                            continue;
                        } else {
                            return Poll::Pending;
                        }
                    };

                    phase.set(Phase::Drain(Some(res)));
                }
                Project::Drain(response) => {
                    ready!(io.poll_drain(cx)?);

                    let res = response.take().unwrap();

                    io.write(res.version().as_str().as_bytes());
                    io.write(b" ");
                    io.write(res.status().as_str().as_bytes());
                    io.write(b"\r\nDate: ");
                    io.write(&httpdate_now()[..]);
                    io.write(b"\r\nContent-Length: ");
                    io.write(
                        itoa::Buffer::new()
                            .format(res.body().remaining())
                            .as_bytes(),
                    );
                    io.write(b"\r\n");

                    for (key, val) in res.headers() {
                        io.write(key.as_str().as_bytes());
                        io.write(b": ");
                        io.write(val.as_bytes());
                        io.write(b"\r\n");
                    }

                    io.write(b"\r\n");

                    // TODO: wait for HeaderMap::clear
                    // let mut map = mem::take(res.headers_mut());
                    // map.clear();
                    // header_map.replace(map);

                    phase.set(Phase::Flush(io.write_body(res.into_body())));
                }
                Project::Flush(b) => {
                    ready!(io.poll_flush(cx))?;
                    ready!(b.poll_write(io, cx))?;
                    phase.set(Phase::Cleanup);
                }
                Project::Cleanup => {
                    // this phase exists to ensure all shared bytes is dropped, thus can be
                    // reclaimed
                    io.clear_reclaim();
                    phase.set(Phase::Read);
                }
            }
        }
    }
}

fn io_err<E: Into<Box<dyn std::error::Error + Send + Sync>>>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e)
}

impl<IO, S, F> std::fmt::Debug for Connection<IO, S, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").finish_non_exhaustive()
    }
}

impl<IO, S> Future for Connection<IO, S, S::Future>
where
    IO: AsyncIoRead + AsyncIoWrite,
    S: HttpService<Error: Into<Box<dyn std::error::Error + Send + Sync>>>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if let Err(err) = ready!(self.try_poll(cx)) {
            eprintln!("{err}")
        }

        println!(">> Disconnected");
        Poll::Ready(())
    }
}

// ===== Projection =====

enum Project<'a, F> {
    Read,
    Service(Pin<&'a mut F>),
    Drain(&'a mut Option<Response>),
    Flush(&'a mut BodyWrite),
    Cleanup,
}

impl<F> Phase<F> {
    fn project(self: Pin<&mut Self>) -> Project<'_, F> {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe {
            match self.get_unchecked_mut() {
                Self::Read => Project::Read,
                Self::Service(f) => Project::Service(Pin::new_unchecked(f)),
                Self::Drain(r) => Project::Drain(r),
                Self::Flush(b) => Project::Flush(b),
                Self::Cleanup => Project::Cleanup,
            }
        }
    }
}
