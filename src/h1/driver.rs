use std::{
    pin::Pin,
    sync::Arc,
    task::{Poll, ready},
};
use tcio::io::{AsyncIoRead, AsyncIoWrite};

use super::{
    io::IoBuffer,
    parser::{Header, Reqline},
    proto::{self, HttpState},
};
use crate::{
    body::{Body, BodyWrite},
    headers::HeaderMap,
    request::Request,
    response::Response,
    service::HttpService,
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

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

                    let reqline = match Reqline::parse_chunk(bytes).into_poll_result()? {
                        Poll::Ready(ok) => ok,
                        Poll::Pending => {
                            // WARN: why `io.poll_read()` here while at the start will also `io.poll_read()` ?
                            // ready!(io.poll_read(cx)?);
                            continue;
                        }
                    };

                    let state = match header_map.take() {
                        Some(headers) => {
                            debug_assert!(headers.is_empty());
                            HttpState::with_headers(reqline, headers)
                        },
                        None => HttpState::new(reqline),
                    };
                    phase.set(Phase::Header(Some(state)));
                }
                PhaseProject::Header(state) => {
                    // TODO: create custom error type that can generate Response

                    let state_mut = state.as_mut().unwrap();

                    loop {
                        let bytes = io.read_buffer_mut();

                        match Header::parse_chunk(bytes).into_poll_result()? {
                            Poll::Ready(Some(header)) => state_mut.insert_header(header)?,
                            Poll::Ready(None) => break,
                            Poll::Pending => {
                                // TODO: limit buffer size
                                ready!(io.poll_read(cx)?);
                                continue;
                            }
                        }
                    }

                    // ===== Service =====

                    let content_len = state_mut.try_content_len()?.unwrap_or(0);
                    let parts = state.take().unwrap().build_parts()?;
                    let body = if io.read_buffer_mut().len() == content_len as usize {
                        // all body have been read, use standalone representation
                        Body::new(io.read_buffer_mut().split())
                    } else {
                        // `IoBuffer` remaining is only calculated excluding the already read body
                        let Some(remaining_body_len) = content_len.checked_sub(io.read_buffer_mut().len() as u64)
                        else {
                            return Poll::Ready(Err("content-length is less than body".into()));
                        };
                        io.set_remaining(remaining_body_len);
                        Body::from_handle(io.handle(), remaining_body_len)
                    };

                    let request = Request::from_parts(parts, body);

                    let service_future = service.call(request);
                    phase.set(Phase::Service(service_future));
                }
                PhaseProject::Service(service_future) => match service_future.poll(cx) {
                    Poll::Ready(Ok(ok)) => {
                        phase.set(Phase::Drain(Some(ok)));
                    }
                    Poll::Ready(Err(err)) => {
                        return Poll::Ready(Err(err.into()));
                    }
                    Poll::Pending => {
                        if ready!(io.poll_io_wants(cx))? {
                            continue;
                        } else {
                            return Poll::Pending;
                        }
                    }
                },
                PhaseProject::Drain(response) => {
                    ready!(io.poll_drain(cx)?);

                    let (mut parts, body) = response.take().unwrap().into_parts();

                    proto::write_response(&parts, io.write_buffer_mut(), body.remaining() as _);

                    parts.headers.clear();
                    header_map.replace(parts.headers);

                    phase.set(Phase::Flush(body.into_writer()));
                }
                PhaseProject::Flush(body_writer) => {
                    ready!(io.poll_flush(cx))?;
                    ready!(body_writer.poll_write(io, cx))?;
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

