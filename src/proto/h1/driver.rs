use std::{
    pin::Pin,
    sync::Arc,
    task::{Poll, ready},
};
use tcio::io::{AsyncIoRead, AsyncIoWrite};

use super::io::IoBuffer;
use crate::{
    body::Body,
    h1::{
        parser::{Header, Reqline},
        proto::HttpState,
    },
    headers::HeaderMap,
    http::httpdate_now,
    proto::h1::io::BodyWrite,
    request::Request,
    response::Response,
    service::HttpService,
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

macro_rules! try_ready {
    ($e:expr) => {
        match ready!($e) {
            Ok(ok) => ok,
            Err(err) => return Poll::Ready(Err(err.into())),
        }
    };
}

pub struct Connection<IO, S, F> {
    io: IoBuffer<IO>,
    header_map: Option<HeaderMap>,
    phase: Phase<F>,
    service: Arc<S>,
}

type ConnectionProject<'a, IO, F, S> = (
    &'a mut IoBuffer<IO>,
    &'a mut Option<HeaderMap>,
    Pin<&'a mut Phase<F>>,
    &'a mut Arc<S>,
);

enum Phase<F> {
    Reqline,
    Header(Option<HttpState>),
    Service(F),
    Drain(Option<Response>),
    Flush(BodyWrite),
    Cleanup,
}

enum PhaseProject<'a, F> {
    Reqline,
    Header(&'a mut Option<HttpState>),
    Service(Pin<&'a mut F>),
    Drain(&'a mut Option<Response>),
    Flush(&'a mut BodyWrite),
    Cleanup,
}

impl<IO, S> Connection<IO, S, S::Future>
where
    S: HttpService<Error: Into<BoxError>>,
{
    pub fn new(io: IO, service: Arc<S>) -> Self {
        Self {
            header_map: None,
            io: IoBuffer::new(io),
            phase: Phase::Reqline,
            service,
        }
    }

    fn try_poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Result<(), BoxError>>
    where
        IO: AsyncIoRead + AsyncIoWrite,
    {
        let (io, header_map, mut phase, service) = self.project();

        loop {
            match phase.as_mut().project() {
                PhaseProject::Reqline => {
                    let read = ready!(io.poll_read(cx)?);
                    if read == 0 {
                        return Poll::Ready(Ok(()));
                    }

                    let bytes = io.read_buffer_mut();

                    // TODO: send error response before disconnect

                    let reqline = match Reqline::matches(bytes)? {
                        Poll::Ready(ok) => ok,
                        Poll::Pending => {
                            ready!(io.poll_read(cx)?);
                            continue;
                        }
                    };
                    let state = match header_map.take() {
                        Some(headers) => HttpState::with_cached_headers(reqline, headers),
                        None => HttpState::new(reqline),
                    };
                    phase.set(Phase::Header(Some(state)));
                }
                PhaseProject::Header(state) => {
                    // TODO: send error response before disconnect

                    let state_mut = state.as_mut().unwrap();

                    loop {
                        let bytes = io.read_buffer_mut();

                        match Header::matches(bytes)? {
                            Poll::Ready(Some(header)) => state_mut.add_header(header)?,
                            Poll::Ready(None) => break,
                            Poll::Pending => {
                                try_ready!(io.poll_read(cx));
                                continue;
                            }
                        }
                    }

                    // ===== Service =====

                    let content_len = state_mut.content_len().unwrap_or(0);
                    let partial_body = io.read_buffer_mut().split();

                    // buffer is empty, so reserve will not need to copy any data if
                    // allocation required
                    io.read_buffer_mut().reserve(content_len as _);

                    // `IoBuffer` remaining is only calculated excluding the already read body
                    let Some(remaining_body_len) = content_len.checked_sub(partial_body.len() as _)
                    else {
                        return Poll::Ready(Err("content-length is less than body".into()));
                    };
                    io.set_remaining(remaining_body_len);

                    let parts = state.take().unwrap().build_parts()?;
                    let body = Body::from_handle(io.get_handle(), content_len, partial_body);
                    let request = Request::from_parts(parts, body);

                    let f = service.call(request);
                    phase.set(Phase::Service(f));
                }
                PhaseProject::Service(f) => {
                    let Poll::Ready(res) = f.poll(cx).map_err(<_>::into)? else {
                        if ready!(io.poll_io_wants(cx))? {
                            continue;
                        } else {
                            return Poll::Pending;
                        }
                    };

                    phase.set(Phase::Drain(Some(res)));
                }
                PhaseProject::Drain(response) => {
                    ready!(io.poll_drain(cx)?);

                    let mut res = response.take().unwrap();

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

                    let mut map = std::mem::take(res.headers_mut());
                    map.clear();
                    header_map.replace(map);

                    phase.set(Phase::Flush(io.write_body(res.into_body())));
                }
                PhaseProject::Flush(b) => {
                    ready!(io.poll_flush(cx))?;
                    ready!(b.poll_write(io, cx))?;
                    phase.set(Phase::Cleanup);
                }
                PhaseProject::Cleanup => {
                    // this phase exists to ensure all shared bytes is dropped, thus can be
                    // reclaimed
                    io.clear_reclaim();
                    phase.set(Phase::Reqline);
                }
            }
        }
    }
}

impl<IO, S> Future for Connection<IO, S, S::Future>
where
    IO: AsyncIoRead + AsyncIoWrite,
    S: HttpService<Error: Into<BoxError>>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        if let Err(err) = ready!(self.try_poll(cx)) {
            eprintln!("{err}")
        }
        Poll::Ready(())
    }
}

impl<IO, S, F> std::fmt::Debug for Connection<IO, S, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").finish_non_exhaustive()
    }
}

// ===== Projection =====

impl<IO, S, F> Connection<IO, S, F> {
    fn project(self: Pin<&mut Self>) -> ConnectionProject<'_, IO, F, S> {
        // SAFETY: self is pinned, no custom Drop and Unpin
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

impl<F> Phase<F> {
    fn project(self: Pin<&mut Self>) -> PhaseProject<'_, F> {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe {
            match self.get_unchecked_mut() {
                Self::Reqline => PhaseProject::Reqline,
                Self::Header(h) => PhaseProject::Header(h),
                Self::Service(f) => PhaseProject::Service(Pin::new_unchecked(f)),
                Self::Drain(r) => PhaseProject::Drain(r),
                Self::Flush(b) => PhaseProject::Flush(b),
                Self::Cleanup => PhaseProject::Cleanup,
            }
        }
    }
}
