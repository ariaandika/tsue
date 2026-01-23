use std::io::IoSlice;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Poll, ready};
use std::{io, mem};
use tcio::bytes::{Buf, BytesMut};
use tcio::io::{AsyncRead, AsyncWrite};

use crate::body::BodyCoder;
use crate::body::handle::SendHandle;
use crate::body::{Body, EncodedBuf};
use crate::h1::parser::{parse_header_chunk, parse_reqline_chunk};
use crate::headers::HeaderMap;
use crate::http::Request;
use crate::proto::{self, Header, HttpContext, HttpState, Reqline};
use crate::service::HttpService;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const MAX_FIELD_CAP: usize = 4 * 1024;
const DEFAULT_BUFFER_CAP: usize = 1024;

/// Read bytes from IO into buffer.
///
/// Handle zero read and max buffer length.
macro_rules! io_read {
    ($io:expr, $buffer:ident, $cx:expr) => {
        let read = ready!($io.as_mut().poll_read(&mut *$buffer, $cx))?;
        if read == 0 {
            return Poll::Ready(Ok(()));
        }
        if $buffer.len() > MAX_FIELD_CAP {
            return Poll::Ready(Err("excessive field size".into()));
        }
    };
    ($io:expr, $buffer:ident, $shared:expr, $cx:ident else $el:expr) => {{
        match ready!($io.as_mut().poll_read(&mut *$buffer, $cx)) {
            Ok(read) => {
                if read == 0 {
                    $shared.set_io_error(io::ErrorKind::ConnectionAborted.into(), $cx);
                    $el
                }
            },
            Err(err) => {
                $shared.set_io_error(err, $cx);
                $el
            },
        }
    }};
}

pub struct Connection<IO, S, B, D, F> {
    io: IO,
    shared: SendHandle,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    header_map: HeaderMap,
    phase: Phase<B, D, F>,
    service: Arc<S>,
    // === per request ===
    context: HttpContext,
    decoder: BodyCoder,
}

type ConnectionProject<'a, IO, S, B, D, F> = (
    Pin<&'a mut IO>,
    &'a mut SendHandle,
    &'a mut BytesMut,
    &'a mut BytesMut,
    &'a mut HeaderMap,
    Pin<&'a mut Phase<B, D, F>>,
    &'a mut Arc<S>,
    &'a mut HttpContext,
    &'a mut BodyCoder,
);

enum Phase<B, D, F> {
    Reqline(Option<Reqline>),
    Service {
        service: F,
    },
    Response {
        body: B,
        encoder: BodyCoder,
        chunk: Option<EncodedBuf<D>>,
    },
    ResponseNoBody,
    Cleanup,
}

enum PhaseProject<'a, B, D, F> {
    Reqline(&'a mut Option<Reqline>),
    Service {
        service: Pin<&'a mut F>,
    },
    Response {
        body: Pin<&'a mut B>,
        encoder: &'a mut BodyCoder,
        chunk: &'a mut Option<EncodedBuf<D>>,
    },
    ResponseNoBody,
    Cleanup,
}

impl<IO, S, B> Connection<IO, S, B, B::Data, S::Future>
where
    S: HttpService<ResBody = B, Error: Into<BoxError>>,
    B: Body,
{
    pub fn new(io: IO, service: Arc<S>) -> Self {
        Self {
            io,
            shared: SendHandle::new(),
            header_map: HeaderMap::with_capacity(16),
            read_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            write_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            phase: Phase::Reqline(None),
            service,
            context: HttpContext::default(),
            decoder: BodyCoder::from_len(Some(0)),
        }
    }

    fn try_poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Result<(), BoxError>>
    where
        IO: AsyncRead + AsyncWrite,
        B: Body,
        B::Error: std::error::Error + Send + Sync + 'static,
    {
        // TODO: create custom error type that can generate Response

        let (
            mut io,
            shared,
            read_buffer,
            write_buffer,
            header_map,
            mut phase,
            service,
            context,
            decoder,
        ) = self.project();

        loop {
            match phase.as_mut().project() {
                PhaseProject::Reqline(reqline) => {
                    if reqline.is_none() {
                        match parse_reqline_chunk(&mut *read_buffer).into_poll_result()? {
                            Poll::Ready(ok) => {
                                *reqline = Some(ok);
                            },
                            Poll::Pending => {
                                io_read!(io, read_buffer, cx);
                                continue;
                            }
                        }
                    }

                    loop {
                        match parse_header_chunk(read_buffer).into_poll_result()? {
                            Poll::Ready(Some(Header { name, value })) => {
                                proto::insert_header(header_map, name, value)?;
                            }
                            Poll::Ready(None) => break,
                            Poll::Pending => {
                                io_read!(io, read_buffer, cx);
                                continue;
                            }
                        }
                    }

                    // ===== Request =====
                    let Some(reqline) = reqline.take() else {
                        // SAFETY: checked at the start of the arm
                        unsafe { std::hint::unreachable_unchecked() }
                    };

                    let state = HttpState::new(reqline, mem::take(header_map));

                    *decoder = state.build_decoder()?;
                    *context = state.build_context()?;

                    let parts = state.build_parts()?;
                    let body = decoder.build_body(read_buffer, shared, cx);
                    let request = Request::from_parts(parts, body);

                    let service = service.call(request);
                    phase.set(Phase::Service { service });
                }
                PhaseProject::Service { service } => {
                    let response = match service.poll(cx) {
                        Poll::Ready(Ok(ok)) => ok,
                        Poll::Ready(Err(err)) => {
                            return Poll::Ready(Err(err.into()));
                        }
                        Poll::Pending => {
                            read_buffer.reserve(512);
                            while shared.poll_read(read_buffer, decoder, cx).is_pending() {
                                io_read!(io, read_buffer, shared, cx else {
                                    break
                                });
                            }
                            return Poll::Pending;
                        }
                    };

                    // ===== Response ======
                    let (parts, body) = response.into_parts();
                    let is_res_body = !body.is_end_stream();

                    let encoder = BodyCoder::from_len(body.size_hint().1);
                    let coding = is_res_body.then_some(encoder.coding());
                    proto::write_response_head(&parts, &mut *write_buffer, coding);

                    // reuse header map allocation
                    let mut headers = parts.headers;
                    headers.clear();
                    *header_map = headers;

                    let next_phase = if context.is_res_body_allowed && is_res_body {
                        Phase::Response { body, encoder, chunk: None }
                    } else {
                        Phase::ResponseNoBody
                    };

                    phase.set(next_phase);
                },
                PhaseProject::ResponseNoBody => {
                    // if recv handle is still alive, user may read the body concurrently
                    if shared.poll_close(read_buffer, decoder, cx)?.is_pending() {
                        io_read!(io, read_buffer, shared, cx else {});
                    }

                    ready!(io.as_mut().poll_write_all_buf(&mut *write_buffer, cx))?;
                    phase.set(Phase::Cleanup);
                },
                PhaseProject::Response { mut body, encoder, chunk } => {
                    // if recv handle is still alive, user may read the body concurrently
                    if shared.poll_close(read_buffer, decoder, cx)?.is_pending() {
                        io_read!(io, read_buffer, shared, cx else {});
                    }

                    ready!(io.as_mut().poll_write_all_buf(&mut *write_buffer, cx))?;

                    loop {
                        match chunk {
                            Some(EncodedBuf { header, chunk: chunk_mut, trail }) => {
                                let mut buf = header.chain(chunk_mut).chain(trail);
                                while buf.has_remaining() {
                                    let mut io_slice = [IoSlice::new(&[]);3];
                                    buf.chunks_vectored(&mut io_slice);
                                    let read = ready!(
                                        io.as_mut().poll_write_vectored(&io_slice, cx)
                                    )?;
                                    buf.advance(read);
                                }
                                *chunk = None;
                            },
                            None => {
                                if body.is_end_stream() {
                                    break;
                                }

                                let Some(frame) = ready!(Body::poll_data(body.as_mut(), cx)?) else {
                                    break;
                                };

                                // TODO: user message body trailer is discarded
                                if let Ok(data) = frame.into_data() {
                                    let encoded = encoder.encode_chunk(data, write_buffer, body.is_end_stream())?;
                                    *chunk = Some(encoded);
                                }
                            },
                        }
                    }

                    phase.set(Phase::Cleanup);
                }
                PhaseProject::Cleanup => {
                    // if recv handle is still alive, user may read the body concurrently, this will
                    // either block subsequent request, or fail because connection dropped
                    let keep_alive = match shared.poll_close(read_buffer, decoder, cx)? {
                        Poll::Ready(Some(keep_alive)) => keep_alive,
                        Poll::Ready(None) => return Poll::Pending,
                        Poll::Pending => {
                            io_read!(io, read_buffer, shared, cx else {});
                            continue;
                        }
                    };

                    if !(context.is_keep_alive && keep_alive) {
                        return Poll::Ready(Ok(()));
                    }

                    phase.set(Phase::Reqline(None));
                }
            }
        }
    }
}

impl<IO, S, B> Future for Connection<IO, S, B, B::Data, S::Future>
where
    IO: AsyncRead + AsyncWrite,
    S: HttpService<ResBody = B, Error: Into<BoxError>>,
    B: Body,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        if let Err(err) = ready!(self.try_poll(cx)) {
            eprintln!("Connection error: {err}")
        }
        println!("Client closed");
        Poll::Ready(())
    }
}

impl<IO, S, B, D, F> std::fmt::Debug for Connection<IO, S, B, D, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").finish_non_exhaustive()
    }
}

// ===== Projection =====

impl<IO, S, B, D, F> Connection<IO, S, B, D, F> {
    fn project(self: Pin<&mut Self>) -> ConnectionProject<'_, IO, S, B, D, F> {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe {
            let me = self.get_unchecked_mut();
            (
                Pin::new_unchecked(&mut me.io),
                &mut me.shared,
                &mut me.read_buffer,
                &mut me.write_buffer,
                &mut me.header_map,
                Pin::new_unchecked(&mut me.phase),
                &mut me.service,
                &mut me.context,
                &mut me.decoder,
            )
        }
    }
}

impl<B, D, F> Phase<B, D, F> {
    fn project(self: Pin<&mut Self>) -> PhaseProject<'_, B, D, F> {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe {
            match self.get_unchecked_mut() {
                Self::Reqline(reqline) => PhaseProject::Reqline(reqline),
                Self::Service { service } => PhaseProject::Service {
                    service: Pin::new_unchecked(service),
                },
                Self::Response {
                    body,
                    encoder,
                    chunk,
                } => PhaseProject::Response {
                    body: Pin::new_unchecked(body),
                    encoder,
                    chunk,
                },
                Self::ResponseNoBody => PhaseProject::ResponseNoBody,
                Self::Cleanup => PhaseProject::Cleanup,
            }
        }
    }
}

