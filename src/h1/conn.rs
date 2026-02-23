use std::mem;
use std::pin::Pin;
use std::task::Poll::{self, *};
use std::task::ready;
use tcio::bytes::{Buf, BytesMut};
use tcio::io::{AsyncRead, AsyncWrite};

use crate::body::Body;
use crate::h1::body::BodyKind;
use crate::h1::chunked::{ChunkedCoder, EncodedChunk};
use crate::h1::proto::{RequestContext, RequestParser};
use crate::h1::states::Session;
use crate::service::HttpService;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const DEFAULT_BUFFER_CAP: usize = 1024;
const MIN_BODY_DRAIN: u64 = 64 * 1024;

/// HTTP/1.1 Connection.
pub struct Connection<S, IO>
where
    S: HttpService
{
    phase: Phase<S>,
    session: Session,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    service: S,
    io: IO,
}

enum Phase<S>
where
    S: HttpService
{
    Request(RequestParser),
    Service(RequestContext, S::Future),
    ResponseNoBody,
    Response(RequestContext, u64, S::ResBody, Option<<S::ResBody as Body>::Data>),
    ResponseChunk(ChunkedCoder, S::ResBody, Option<EncodedChunk<<S::ResBody as Body>::Data>>),
    Drain(u64),
    Complete,
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
        let Self {
            phase,
            session,
            read_buffer,
            write_buffer,
            service,
            io,
        } = unsafe { self.get_unchecked_mut() };
        // SAFETY: self is pinned
        let mut io = unsafe { Pin::new_unchecked(io) };

        loop {
            match phase {
                Phase::Request(parser) => {
                    let Ready((method, target)) = parser.poll_request(session, &mut *read_buffer)? else {
                        let read = ready!(io.as_mut().poll_read(&mut *read_buffer, cx)?);
                        if read == 0 {
                            return Ready(Ok(()))
                        }
                        continue;
                    };
                    let (request, context) = RequestContext::new(method, target, session, read_buffer, cx)?;
                    *phase = Phase::Service(context, service.call(request));
                }
                Phase::Service(context, future) => {
                    // SAFETY: `self` is pinned, thus `self.phase` is also pinned
                    let future = unsafe { Pin::new_unchecked(future) };
                    let response = match future.poll(cx) {
                        Ready(Ok(ok)) => ok,
                        Ready(Err(err)) => return Ready(Err(err.into())),
                        Pending => {
                            read_buffer.reserve(DEFAULT_BUFFER_CAP);
                            while context.poll_read(session, read_buffer, cx) {
                                match ready!(io.as_mut().poll_read(&mut *read_buffer, cx)) {
                                    Ok(0) => {
                                        session.shared.set_io_error(
                                            std::io::ErrorKind::ConnectionAborted.into(),
                                            cx
                                        );
                                        break;
                                    },
                                    Ok(_) => {},
                                    Err(err) => {
                                        session.shared.set_io_error(err, cx);
                                        break;
                                    },
                                };
                            }
                            continue;
                        }
                    };

                    let Phase::Service(context, _) = mem::replace(phase, Phase::ResponseNoBody) else {
                        unreachable!()
                    };

                    *phase = match context.build_response_writer(response, session, write_buffer) {
                        Some((body, BodyKind::ContentLength(len))) => Phase::Response(context, len, body, None),
                        Some((body, BodyKind::Chunked(encoder))) => Phase::ResponseChunk(encoder, body, None),
                        None => Phase::ResponseNoBody,
                    };
                }
                Phase::ResponseNoBody => {
                    ready!(io.as_mut().poll_write_all_buf(&mut *write_buffer, cx)?);
                    *phase = Phase::Complete;
                }
                Phase::Response(context, remaining, body, data_mut) => {
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

                    if context.decoder.has_remaining() {
                        match context.decoder.remaining() {
                            Some(remaining) if remaining <= MIN_BODY_DRAIN => {
                                *phase = Phase::Drain(remaining);
                            },
                            _ => return Ready(Ok(())),
                        }
                    } else {
                        *phase = Phase::Complete;
                    }
                }
                Phase::ResponseChunk(encoder, body, data_mut) => {
                    ready!(io.as_mut().poll_write_all_buf(&mut *write_buffer, cx)?);

                    loop {
                        while let Some(EncodedChunk { data, trail }) = data_mut {
                            let mut chunks = write_buffer.chain(data).chain(trail);
                            let mut io_slice = [std::io::IoSlice::new(&[]); 8];
                            let cnt = chunks.chunks_vectored(&mut io_slice);
                            let write = ready!(io.as_mut().poll_write_vectored(&io_slice[..cnt], cx)?);
                            chunks.advance(write);
                            if !chunks.has_remaining() {
                                *data_mut = None;
                                break;
                            }
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
                            *data_mut = Some(encoder.encode_chunk(data, body.is_end_stream(), write_buffer));
                        }
                    }

                    *phase = Phase::Complete;
                }
                Phase::Drain(remaining_mut) => {
                    loop {
                        if *remaining_mut == 0 {
                            break;
                        }
                        let read = ready!(io.as_mut().poll_read(&mut *read_buffer, cx)?);
                        if read == 0 {
                            return Ready(Ok(()))
                        }
                        let Some(new_remain) = remaining_mut.checked_sub(read as u64) else {
                            break
                        };
                        *remaining_mut = new_remain;
                        read_buffer.clear();
                    }

                    *phase = Phase::Complete;
                }
                Phase::Complete => {
                    if !session.keep_alive {
                        return Ready(Ok(()));
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

