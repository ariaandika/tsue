use std::mem;
use std::pin::Pin;
use std::task::Poll::{self, *};
use std::task::ready;
use tcio::bytes::{Buf, BytesMut};
use tcio::io::{AsyncRead, AsyncWrite};

use crate::body::Body;
use crate::h1::body::{BodyEncoder, LengthEncoder};
use crate::h1::chunked::{ChunkedCoder, EncodedChunk};
use crate::h1::proto::{RequestContext, poll_request};
use crate::h1::states::Session;
use crate::proto::error::UserError;
use crate::service::HttpService;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const DEFAULT_BUFFER_CAP: usize = 1024;

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
    S: HttpService,
{
    Request,
    Service(RequestContext, S::Future),
    Response(
        RequestContext,
        LengthEncoder,
        S::ResBody,
        Option<<S::ResBody as Body>::Data>,
    ),
    ResponseChunked(
        RequestContext,
        ChunkedCoder,
        S::ResBody,
        Option<EncodedChunk<<S::ResBody as Body>::Data>>,
    ),
    Drain(RequestContext),
    Complete,
}

impl<S, IO> Connection<S, IO>
where
    S: HttpService
{
    pub fn new(service: S, io: IO) -> Self {
        Self {
            phase: Phase::Request,
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
                Phase::Request => {
                    let Ready((parts, mut context)) = poll_request(session, &mut *read_buffer)? else {
                        let read = ready!(io.as_mut().poll_read(&mut *read_buffer, cx)?);
                        if read == 0 {
                            return Ready(Ok(()))
                        }
                        continue;
                    };
                    let request = context.build_request(parts, session, read_buffer, cx);
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
                                let result = match ready!(io.as_mut().poll_read(&mut *read_buffer, cx)) {
                                    Ok(0) => Err(std::io::ErrorKind::ConnectionAborted.into()),
                                    Ok(_) => Ok(()),
                                    Err(err) => Err(err),
                                };
                                if let Err(err) = result {
                                    session.shared.set_io_error(err, cx);
                                    break;
                                }
                            }
                            continue;
                        }
                    };

                    let Phase::Service(context, _) = mem::replace(phase, Phase::Request) else {
                        unreachable!()
                    };

                    let (body, kind) = context.build_response_writer(response, session, write_buffer);
                    *phase = match kind {
                        BodyEncoder::Length(encoder) => Phase::Response(context, encoder, body, None),
                        BodyEncoder::Chunked(encoder) => Phase::ResponseChunked(context, encoder, body, None),
                    };
                }
                Phase::Response(context, encoder, body, data_mut) => {
                    ready!(io.as_mut().poll_write_all_buf(&mut *write_buffer, cx)?);

                    loop {
                        if let Some(data) = data_mut {
                            ready!(io.as_mut().poll_write_all_buf(data, cx)?);
                            *data_mut = None;
                        }

                        if encoder.is_exhausted() {
                            break;
                        }

                        // SAFETY: `self` is pinned, thus `self.phase` is also pinned
                        let body = unsafe { Pin::new_unchecked(&mut *body) };
                        match ready!(body.poll_data(cx)) {
                            Some(Ok(data)) => {
                                *data_mut = Some(encoder.encode(data)?);
                            },
                            None => {
                                // has remaining, but body is exhausted
                                return Ready(Err(UserError::ExcessiveContent.into()));
                            }
                            Some(Err(err)) => return Poll::Ready(Err(err.into())),
                        };
                    }

                    *phase = if context.needs_drain()? {
                        let Phase::Response(context, _, _, _) = mem::replace(phase, Phase::Request) else {
                            unreachable!()
                        };
                        Phase::Drain(context)
                    } else {
                        Phase::Complete
                    };
                }
                Phase::ResponseChunked(context, encoder, body, data_mut) => {
                    ready!(io.as_mut().poll_write_all_buf(&mut *write_buffer, cx)?);

                    loop {
                        while let Some(chunk) = data_mut {
                            let mut chunks = write_buffer.chain(chunk);
                            let mut io_slice = [std::io::IoSlice::new(&[]); 16];
                            let cnt = chunks.chunks_vectored(&mut io_slice);
                            let write = ready!(io.as_mut().poll_write_vectored(&io_slice[..cnt], cx)?);
                            chunks.advance(write);
                            if !chunks.has_remaining() {
                                *data_mut = None;
                                break;
                            }
                        }

                        if encoder.is_eof() {
                            break;
                        }

                        // SAFETY: `self` is pinned, thus `self.phase` is also pinned
                        let mut body = unsafe { Pin::new_unchecked(&mut *body) };
                        let data = match ready!(body.as_mut().poll_data(cx)) {
                            Some(Ok(ok)) => ok,
                            Some(Err(err)) => return Poll::Ready(Err(err.into())),
                            None => break,
                        };

                        *data_mut = Some(encoder.encode_chunk(data, body.is_end_stream(), write_buffer));
                    }

                    // TODO: check for recv shared handle should be dropped

                    *phase = if context.needs_drain()? {
                        let Phase::ResponseChunked(context, _, _, _) = mem::replace(phase, Phase::Request) else {
                            unreachable!()
                        };
                        Phase::Drain(context)
                    } else {
                        Phase::Complete
                    };
                }
                Phase::Drain(context) => {
                    loop {
                        let read = ready!(io.as_mut().poll_read(&mut *read_buffer, cx)?);
                        if read == 0 {
                            return Ready(Ok(()))
                        }
                        if let Ready(()) = context.poll_drain(read) {
                            break
                        }
                    }
                    *phase = Phase::Complete;
                }
                Phase::Complete => {
                    if !session.keep_alive {
                        return Ready(Ok(()));
                    }
                    session.shared.detach();
                    read_buffer.reclaim();
                    *phase = Phase::Request;
                }
            }
        }
    }
}

impl<S, IO> Future for Connection<S, IO>
where
    S: HttpService,
    IO: AsyncRead + AsyncWrite,
{
    type Output = ();

    #[inline]
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

