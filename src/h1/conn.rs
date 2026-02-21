use std::pin::Pin;
use std::task::Poll::{self, *};
use std::task::ready;
use tcio::bytes::{Buf, BytesMut};
use tcio::io::{AsyncRead, AsyncWrite};

use crate::body::Body;
use crate::h1::proto::{BodyKind, ChunkedEncoder, EncodedChunk, Session};
use crate::h1::proto::{RequestParser, RequestState};
use crate::service::HttpService;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const DEFAULT_BUFFER_CAP: usize = 512;

/// HTTP/1.1 Connection.
pub struct Connection<S, IO>
where
    S: HttpService
{
    phase: Phase<S::Future, S::ResBody, <S::ResBody as Body>::Data>,
    session: Session,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    service: S,
    io: IO,
}

enum Phase<F, B, D> {
    Request(RequestParser),
    Service(RequestState, F),
    ResponseNoBody,
    Response(u64, B, Option<D>),
    ResponseChunk(ChunkedEncoder, B, Option<EncodedChunk<D>>),
}

impl<S, IO> Connection<S, IO>
where
    S: HttpService
{
    pub fn new(service: S, io: IO) -> Self {
        Self {
            phase: Phase::Request(RequestParser::new()),
            session: Session::new(),
            read_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            write_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            service,
            io,
        }
    }
}

impl<S, IO> Connection<S, IO>
where
    S: HttpService,
    <S::ResBody as Body>::Error: Into<BoxError>,
    IO: AsyncRead + AsyncWrite,
{
    fn try_poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Result<(), BoxError>> {
        // SAFETY: self is pinned
        let Self { phase, session, read_buffer, write_buffer, service, io } = unsafe { self.get_unchecked_mut() };
        let mut io = unsafe { Pin::new_unchecked(io) };

        loop {
            match phase {
                Phase::Request(parser) => {
                    let Ready(reqline) = parser.poll_request(session, &mut *read_buffer)? else {
                        let read = ready!(io.as_mut().poll_read(&mut *read_buffer, cx)?);
                        if read == 0 {
                            return Ready(Ok(()))
                        }
                        continue;
                    };
                    let (request, state) = RequestState::new(reqline, session, read_buffer, cx)?;
                    let future = service.call(request);
                    *phase = Phase::Service(state, future);
                }
                Phase::Service(state, future) => {
                    // SAFETY: `self` is pinned, thus `self.phase` is also pinned
                    let future = unsafe { Pin::new_unchecked(future) };
                    let response = match future.poll(cx) {
                        Poll::Ready(Ok(ok)) => ok,
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(err.into())),
                        Poll::Pending => {
                            read_buffer.reserve(512);
                            while state.poll_read(session, read_buffer, cx) {
                                ready!(io.as_mut().poll_read(&mut *read_buffer, cx)?);
                            }
                            continue;
                        }
                    };

                    let (body, kind) = state.build_response_writer(response, session, write_buffer);

                    let next_phase = match kind {
                        BodyKind::None => Phase::ResponseNoBody,
                        BodyKind::Exact(writer) => Phase::Response(writer, body, None),
                        BodyKind::Chunked(writer) => Phase::ResponseChunk(writer, body, None),
                    };
                    *phase = next_phase;
                }
                Phase::ResponseNoBody => {
                    ready!(io.as_mut().poll_write_all_buf(&mut *write_buffer, cx)?);
                    *phase = Phase::Request(RequestParser::new());
                }
                Phase::Response(remaining, body, data_mut) => {
                    ready!(io.as_mut().poll_write_all_buf(&mut *write_buffer, cx)?);

                    loop {
                        if let Some(data) = data_mut {
                            let len = data.remaining();
                            ready!(io.as_mut().poll_write_all_buf(data, cx)?);
                            let Some(new_remain) = remaining.checked_sub(len as u64) else {
                                break;
                            };
                            *remaining = new_remain;
                            *data_mut = None;
                        }

                        if *remaining == 0 {
                            break
                        }

                        // SAFETY: `self` is pinned, thus `self.phase` is also pinned
                        let body = unsafe { Pin::new_unchecked(&mut *body) };
                        let data = match ready!(body.poll_data(cx)) {
                            Some(Ok(ok)) => ok,
                            Some(Err(err)) => return Poll::Ready(Err(err.into())),
                            None => break,
                        };

                        // NOTE: currently trailer from user are dropped
                        if let Ok(data) = data.into_data() {
                            *data_mut = Some(data);
                        }
                    }

                    *phase = Phase::Request(RequestParser::new());
                }
                Phase::ResponseChunk(encoder, body, data_mut) => {
                    loop {
                        if let Some(EncodedChunk { chunk, trail }) = data_mut {
                            let chunk = write_buffer.chain(chunk).chain(trail);
                            ready!(io.as_mut().poll_write_buf_vectored(chunk, cx)?);
                            *data_mut = None;
                        }

                        // SAFETY: `self` is pinned, thus `self.phase` is also pinned
                        let mut body = unsafe { Pin::new_unchecked(&mut *body) };
                        let data = match ready!(body.as_mut().poll_data(cx)) {
                            Some(Ok(ok)) => ok,
                            Some(Err(err)) => return Poll::Ready(Err(err.into())),
                            None => break,
                        };

                        // NOTE: currently trailer from user are dropped
                        if let Ok(data) = data.into_data() {
                            *data_mut = Some(encoder.encode_chunk(data, write_buffer, body.is_end_stream()));
                        }
                    }

                    *phase = Phase::Request(RequestParser::new());
                }
            }
        }
    }
}

impl<S, IO> Future for Connection<S, IO>
where
    S: HttpService,
    <S::ResBody as Body>::Error: Into<BoxError>,
    IO: AsyncRead + AsyncWrite,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        match ready!(self.as_mut().try_poll(cx)) {
            Ok(()) => {
                println!("connection closed");
            }
            Err(err) => {
                eprintln!("connection aborted: {err}");
            }
        }
        Poll::Ready(())
    }
}

impl<S, IO> std::fmt::Debug for Connection<S, IO>
where
    S: HttpService
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").finish_non_exhaustive()
    }
}

