use std::mem;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Poll, ready};
use tcio::bytes::{Buf, BytesMut};
use tcio::io::{AsyncIoRead, AsyncIoWrite};

use super::parser::{Header, Reqline};
use crate::body::Body;
use crate::body::handle::Shared;
use crate::headers::HeaderMap;
use crate::body::BodyCoder;
use crate::proto::{self, HttpContext, HttpState};
use crate::http::Request;
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
    shared: Shared,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    header_map: HeaderMap,
    phase: Phase<B, F>,
    service: Arc<S>,
}

type ConnectionProject<'a, IO, S, B, F> = (
    &'a mut IO,
    &'a mut Shared,
    &'a mut BytesMut,
    &'a mut BytesMut,
    &'a mut HeaderMap,
    Pin<&'a mut Phase<B, F>>,
    &'a mut Arc<S>,
);

enum Phase<B, F> {
    Reqline {
        want_read: bool,
    },
    Header(Reqline),
    Service {
        context: HttpContext,
        decoder: BodyCoder,
        service: F,
    },
    ResHeader {
        body: B,
        body_encoder: BodyCoder,
    },
    ResBody {
        body: B,
        body_encoder: BodyCoder,
    },
    Cleanup,
    Placeholder,
}

enum PhaseProject<'a, B, F> {
    Reqline {
        want_read: &'a mut bool,
    },
    Header,
    Service {
        context: &'a mut HttpContext,
        body_encoder: &'a mut BodyCoder,
        service: Pin<&'a mut F>,
    },
    ResHeader,
    ResBody {
        body: Pin<&'a mut B>,
        body_encoder: &'a mut BodyCoder,
    },
    Cleanup,
}

impl<IO, S, B> Connection<IO, S, B, S::Future>
where
    S: HttpService<ResBody = B, Error: Into<BoxError>>,
{
    pub fn new(io: IO, service: Arc<S>) -> Self {
        Self {
            io,
            shared: Shared::new(),
            header_map: HeaderMap::new(),
            read_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            write_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            phase: Phase::Reqline { want_read: true },
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

        let (io, shared, read_buffer, write_buffer, header_map, mut phase, service) = self.project();

        loop {
            match phase.as_mut().project() {
                PhaseProject::Reqline { want_read } => {
                    // it is possible that subsequent request bytes may already in buffer when
                    // reading request body because of request pipelining
                    //
                    // but if `parse_chunk` returns pending, it will also put bytes in the buffer
                    //
                    // thus explicit `want_read` flag is necessary
                    if *want_read {
                        io_read!(io.poll_read_buf(read_buffer, cx));
                    }

                    let reqline = match Reqline::parse_chunk(read_buffer).into_poll_result()? {
                        Poll::Ready(ok) => ok,
                        Poll::Pending => {
                            *want_read = true;
                            continue;
                        }
                    };

                    header_map.reserve(16);
                    phase.set(Phase::Header(reqline));
                }
                PhaseProject::Header => {
                    loop {
                        match Header::parse_chunk(read_buffer).into_poll_result()? {
                            Poll::Pending => {
                                io_read!(io.poll_read_buf(read_buffer, cx));
                                continue;
                            }
                            Poll::Ready(Some(Header { name, value })) => {
                                proto::insert_header(header_map, name, value)?;
                            }
                            Poll::Ready(None) => break,
                        };

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
                    let body = decoder.build_body(read_buffer, shared, cx);

                    let request = Request::from_parts(parts, body);

                    let service = service.call(request);
                    phase.set(Phase::Service { context, decoder, service });
                }
                PhaseProject::Service { context: _, body_encoder: decoder, service } => {
                    let response = match service.poll(cx) {
                        Poll::Ready(Ok(ok)) => ok,
                        Poll::Ready(Err(err)) => {
                            return Poll::Ready(Err(err.into()));
                        }
                        Poll::Pending => {
                            let _ = shared.poll_read(read_buffer, decoder, io, cx);
                            return Poll::Pending;
                        }
                    };

                    // ===== Response ======
                    let (parts, body) = response.into_parts();

                    let body_encoder = BodyCoder::from_len(body.size_hint().1);
                    proto::write_response(&parts, write_buffer, &body_encoder.coding());

                    let mut headers = parts.headers;
                    headers.clear();
                    *header_map = headers;

                    phase.set(Phase::ResHeader { body, body_encoder });
                },
                PhaseProject::ResHeader => {
                    ready!(io.poll_write_all_buf(write_buffer, cx))?;
                    let Phase::ResHeader { body, body_encoder } = phase.as_mut().take() else {
                        // SAFETY: pattern matched
                        unsafe { std::hint::unreachable_unchecked() }
                    };
                    phase.set(Phase::ResBody { body, body_encoder });
                }
                PhaseProject::ResBody { body, body_encoder } => {
                    ready!(io.poll_write_all_buf(write_buffer, cx))?;

                    let Some(frame) = ready!(Body::poll_data(body, cx)?) else {
                        phase.set(Phase::Cleanup);
                        continue;
                    };

                    if let Ok(mut data) = frame.into_data() {
                        let len = data.remaining();
                        ready!(body_encoder.encode_chunk(data.copy_to_bytes(len), &mut *io))?;
                        todo!("statefull body encoding")
                    }
                }
                PhaseProject::Cleanup => {
                    // TODO: drain only if the body is an a drain treshold
                    // ready!(io.poll_drain(cx)?);

                    // `reserve` will try to reclaim buffer, but if the underlying buffer is grow
                    // thus reallocated, and the new allocated capacity is not at least
                    // DEFAULT_BUFFER_CAP, reclaiming does not work, so another reallocation
                    // required
                    //
                    // `clear()` will also ensure this allocation does not need to copy any data
                    read_buffer.clear();
                    read_buffer.reserve(DEFAULT_BUFFER_CAP);

                    phase.set(Phase::Reqline { want_read: false });
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
                &mut me.shared,
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
                Self::Reqline { want_read } => PhaseProject::Reqline { want_read },
                Self::Header(_) => PhaseProject::Header,
                Self::Service {
                    context,
                    decoder,
                    service,
                } => PhaseProject::Service {
                    context,
                    body_encoder: decoder,
                    service: Pin::new_unchecked(service),
                },
                Self::ResHeader { .. } => PhaseProject::ResHeader,
                Self::ResBody { body, body_encoder } => PhaseProject::ResBody {
                    body: Pin::new_unchecked(body),
                    body_encoder,
                },
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

