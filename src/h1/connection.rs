use std::mem;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Poll, ready};
use tcio::bytes::BytesMut;
use tcio::io::{AsyncIoRead, AsyncIoWrite};

use super::parser::{Header, Reqline};
use crate::body::{Body, BodyWrite};
use crate::headers::{HeaderMap, HeaderName, HeaderValue};
use crate::http::spec;
use crate::http::spec::ProtoError;
use crate::http::spec::{HttpContext, HttpState};
use crate::http::{Request, Response};
use crate::service::HttpService;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const MAX_FIELD_CAP: usize = 4 * 1024;
const DEFAULT_BUFFER_CAP: usize = 512;

/// Read bytes from IO into buffer.
macro_rules! io_read {
    ($io:ident.$read:ident($buffer:ident, $cx:expr)) => {
        let read = ready!($io.$read($buffer, $cx)?);
        if read == 0 {
            return Poll::Ready(Ok(()));
        }
        if $buffer.len() > MAX_FIELD_CAP {
            return Poll::Ready(Err("excessive field size".into()));
        }
    };
}

pub struct Connection<IO, S, B, F> {
    io: IO,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    header_map: HeaderMap,
    phase: Phase<B, F>,
    service: Arc<S>,
}

type ConnectionProject<'a, IO, S, B, F> = (
    &'a mut IO,
    &'a mut BytesMut,
    &'a mut BytesMut,
    &'a mut HeaderMap,
    Pin<&'a mut Phase<B, F>>,
    &'a mut Arc<S>,
);

enum Phase<B, F> {
    Reqline { is_parse_pending: bool },
    Header(Reqline),
    Service { context: HttpContext, service: F },
    Drain(Response<B>),
    Flush(B),
    Cleanup,
    Placeholder,
}

enum PhaseProject<'a, B, F> {
    Reqline {
        is_parse_pending: &'a mut bool,
    },
    Header(&'a mut Reqline),
    Service {
        context: &'a mut HttpContext,
        service: Pin<&'a mut F>,
    },
    Drain(&'a mut Response<B>),
    Flush(Pin<&'a mut B>),
    Cleanup,
}

impl<IO, S, B> Connection<IO, S, B, S::Future>
where
    S: HttpService<ResBody = B, Error: Into<BoxError>>,
{
    pub fn new(io: IO, service: Arc<S>) -> Self {
        Self {
            header_map: HeaderMap::new(),
            io,
            read_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            write_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            phase: Phase::Reqline { is_parse_pending: false },
            service,
        }
    }

    fn try_poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Result<(), BoxError>>
    where
        IO: AsyncIoRead + AsyncIoWrite,
        B: Body,
        B::Error: std::error::Error + Send + Sync + 'static,
    {
        // TODO: create custom error type that can generate Response

        let (io, read_buffer, write_buffer, header_map, mut phase, service) = self.project();

        loop {
            match phase.as_mut().project() {
                PhaseProject::Reqline { is_parse_pending } => {
                    // it is possible that subsequent request bytes may already in buffer when
                    // reading request body because of request pipelining
                    //
                    // but if `parse_chunk` returns pending, it will also put bytes in buffer
                    //
                    // thus `is_parse_pending` flag is necessary
                    if read_buffer.is_empty() | *is_parse_pending {
                        io_read!(io.poll_read_buf(read_buffer, cx));
                    }

                    let reqline = match Reqline::parse_chunk(read_buffer).into_poll_result()? {
                        Poll::Ready(ok) => ok,
                        Poll::Pending => {
                            *is_parse_pending = true;
                            continue;
                        }
                    };

                    header_map.reserve(16);
                    phase.set(Phase::Header(reqline));
                }
                PhaseProject::Header(_) => {
                    loop {
                        let header = match Header::parse_chunk(read_buffer).into_poll_result()? {
                            Poll::Ready(Some(ok)) => ok,
                            Poll::Ready(None) => break,
                            Poll::Pending => {
                                io_read!(io.poll_read_buf(read_buffer, cx));
                                continue;
                            }
                        };

                        let Header { mut name, value } = header;
                        name.make_ascii_lowercase();
                        header_map.append(
                            HeaderName::from_bytes_lowercase(name)?,
                            HeaderValue::from_bytes(value)?,
                        );

                        if header_map.len() > spec::MAX_HEADERS {
                            return Poll::Ready(Err(ProtoError::TooManyHeaders.into()));
                        }
                    }

                    // ===== Service =====

                    let Phase::Header(reqline) = phase.as_mut().take() else {
                        // SAFETY: pattern matched
                        unsafe { std::hint::unreachable_unchecked() }
                    };

                    let state = HttpState::new(reqline, mem::take(header_map));
                    let context = state.build_context()?;
                    let decoder = state.build_decoder()?;
                    let parts = state.build_parts()?;

                    // FIXME: create shared Handle where Incoming 
                    // let handle = io.handle();
                    // let body = decoder.build_body(io.read_buffer_mut(), &handle);

                    // let request = Request::from_parts(parts, body);
                    //
                    // let service = service.call(request);
                    // phase.set(Phase::Service { context, service });

                    todo!()
                }
                PhaseProject::Service { context: _, service } => match service.poll(cx) {
                    Poll::Ready(Ok(ok)) => {
                        phase.set(Phase::Drain(ok));
                    }
                    Poll::Ready(Err(err)) => {
                        return Poll::Ready(Err(err.into()));
                    }
                    Poll::Pending => {
                        todo!()
                        // let _ = io.poll_io_wants(cx)?;
                        // return Poll::Pending;
                    }
                },
                PhaseProject::Drain(_) => {
                    // TODO: drain only if the body is an a treshold
                    // ready!(io.poll_drain(cx)?);

                    let Phase::Drain(response) = phase.as_mut().take() else {
                        // SAFETY: pattern matched
                        unsafe { std::hint::unreachable_unchecked() }
                    };

                    let (mut parts, body) = response.into_parts();

                    // spec::write_response(&parts, io.write_buffer_mut(), body.remaining() as _);
                    todo!();

                    parts.headers.clear();
                    *header_map = parts.headers;

                    phase.set(Phase::Flush(body));
                }
                PhaseProject::Flush(body) => {
                    // ready!(io.poll_flush(cx))?;

                    // ready!(body.poll_write(io, cx))?;
                    ready!(Body::poll_data(body, cx)?);
                    todo!();
                    // Body::poll_data(self: std::pin::Pin<&mut Self>, cx)

                    phase.set(Phase::Cleanup);
                }
                PhaseProject::Cleanup => {
                    // this phase exists to ensure all shared bytes is dropped, thus can be
                    // reclaimed

                    // `reserve` will try to reclaim buffer, but if the underlying buffer is grow
                    // thus reallocated, and the new allocated capacity is not at least
                    // DEFAULT_BUFFER_CAP, reclaiming does not work, so another reallocation
                    // required
                    //
                    // `clear()` will also ensure this allocation does not need to copy any data
                    read_buffer.clear();
                    read_buffer.reserve(DEFAULT_BUFFER_CAP);

                    phase.set(Phase::Reqline { is_parse_pending: false });
                }
            }
        }
    }
}

impl<IO, S, B> Future for Connection<IO, S, B, S::Future>
where
    IO: AsyncIoRead + AsyncIoWrite,
    S: HttpService<ResBody = B, Error: Into<BoxError>>,
    B: Body,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        if let Err(err) = ready!(self.try_poll(cx)) {
            eprintln!("{err}")
        }
        Poll::Ready(())
    }
}

impl<IO, S, B, F> std::fmt::Debug for Connection<IO, S, B, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").finish_non_exhaustive()
    }
}

// ===== Projection =====

impl<IO, S, B, F> Connection<IO, S, B, F> {
    fn project(self: Pin<&mut Self>) -> ConnectionProject<'_, IO, S, B, F> {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe {
            let me = self.get_unchecked_mut();
            (
                &mut me.io,
                &mut me.read_buffer,
                &mut me.write_buffer,
                &mut me.header_map,
                Pin::new_unchecked(&mut me.phase),
                &mut me.service,
            )
        }
    }
}

impl<B, F> Phase<B, F> {
    fn project(self: Pin<&mut Self>) -> PhaseProject<'_, B, F> {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe {
            match self.get_unchecked_mut() {
                Self::Reqline { is_parse_pending } => PhaseProject::Reqline { is_parse_pending },
                Self::Header(h) => PhaseProject::Header(h),
                Self::Service { context, service } => PhaseProject::Service {
                    context,
                    service: Pin::new_unchecked(service),
                },
                Self::Drain(r) => PhaseProject::Drain(r),
                Self::Flush(b) => PhaseProject::Flush(Pin::new_unchecked(b)),
                Self::Cleanup => PhaseProject::Cleanup,
                Self::Placeholder => unreachable!(),
            }
        }
    }

    fn take(self: Pin<&mut Self>) -> Self {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe { mem::replace(self.get_unchecked_mut(), Self::Placeholder) }
    }
}

