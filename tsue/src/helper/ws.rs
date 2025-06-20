use base64ct::{Base64, Encoding};
use bytes::{Buf, BytesMut};
use futures_core::Stream;
use http::{
    HeaderMap, HeaderValue, StatusCode,
    header::{CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE},
};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use sha1::{Digest, Sha1};
use std::{
    fmt,
    future::{Ready, poll_fn, ready},
    io::{self, IoSlice},
    pin::Pin,
    task::{Poll, ready},
};

use crate::{
    body::Body,
    common::log,
    helper::WsUpgrade,
    request::{FromRequest, Request},
    response::{IntoResponse, Response},
};

mod frame;

pub use frame::{Frame, OpCode};

/// 64 MiB
const MAX_FRAME_SIZE: usize = 64 << 20;
const MAX_HEADER_SIZE: usize = 14;
const MASK_SIZE: usize = 4;

// https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API/Writing_WebSocket_servers

impl FromRequest for WsUpgrade {
    type Error = WsUpgradeError;
    type Future = Ready<Result<Self,Self::Error>>;

    fn from_request(req: Request) -> Self::Future {
        let headers = req.headers();
        assert_hdr!(headers, CONNECTION, headers.split(',').any(|e|e.trim() == "Upgrade"), "not a connection upgrade");
        assert_hdr!(headers, UPGRADE, b"websocket", "not a websocket upgrade");
        assert_hdr!(headers, SEC_WEBSOCKET_VERSION, b"13", "unsupported websocket version");
        ready(Ok(Self { req }))
    }
}

impl WsUpgrade {
    pub fn upgrade<F, U>(self, handle: F) -> Response
    where
        F: FnOnce(WebSocket) -> U + Send + 'static,
        U: Future<Output = ()> + Send,
    {
        let headers = self.req.headers();
        let key = headers.get(SEC_WEBSOCKET_KEY);
        let derived = HeaderValue::from_bytes(&derive_accept(key.unwrap().as_bytes())).unwrap();

        tokio::spawn(async move {
            let mut req = self.req;
            match hyper::upgrade::on(&mut req).await {
                Ok(io) => handle(WebSocket::new(TokioIo::new(io))).await,
                Err(err) => log!("failed to upgrade websocket: {err}"),
            }
        });

        static DEFAULT_HEADERS: std::sync::LazyLock<HeaderMap> = std::sync::LazyLock::new(||{
            const UPGRADE_RES: HeaderValue = HeaderValue::from_static("Upgrade");
            const WEBSOCKET_RES: HeaderValue = HeaderValue::from_static("websocket");
            const KEY_RES: HeaderValue = HeaderValue::from_static("default");

            let mut headers = HeaderMap::new();
            headers.append(CONNECTION, UPGRADE_RES);
            headers.append(UPGRADE, WEBSOCKET_RES);
            headers.append(SEC_WEBSOCKET_ACCEPT, KEY_RES);
            headers
        });

        let mut res = Response::new(Body::default());
        *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
        *res.headers_mut() = DEFAULT_HEADERS.clone();
        res.headers_mut().insert(SEC_WEBSOCKET_ACCEPT, derived);
        res
    }
}

fn derive_accept(key: &[u8]) -> Vec<u8> {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    let mut sha1 = Sha1::default();
    sha1.update(key);
    sha1.update(WS_GUID);
    let key = sha1.finalize();

    let len = Base64::encoded_len(&key);
    let mut dst = vec![0u8; len];
    let res_len = Base64::encode(&key, &mut dst).unwrap().len();
    if dst.len() > res_len {
        dst.truncate(dst.len() - res_len);
    }

    dst
}

// ===== WebSocket =====

#[derive(Debug)]
pub struct WebSocket {
    io: TokioIo<Upgraded>,
    closed: bool,
    read_buf: BytesMut,
    write_buf: BytesMut,
    fragment: Option<Frame>,
}

impl WebSocket {
    fn new(io: TokioIo<Upgraded>) -> Self {
        Self {
            io,
            closed: false,
            read_buf: BytesMut::with_capacity(512),
            write_buf: BytesMut::with_capacity(512),
            fragment: None,
        }
    }
}

impl WebSocket {
    /// Read for a frame.
    ///
    /// This call collects fragmented messages.
    ///
    /// This call handles ping frame automatically.
    pub async fn read(&mut self) -> io::Result<Frame> {
        let mut frame = self.read_frame().await?;
        let mut is_fin = frame.fin();

        while !is_fin {
            let fragment = self.read_frame().await?;
            is_fin = fragment.fin();

            if fragment.opcode() != OpCode::Continuation {
                return Err(io_err!("fragment opcode is not a continuation"))
            }

            frame.payload_mut().unsplit(fragment.into_payload());
        }

        Ok(frame)
    }

    /// Read for single frame.
    ///
    /// This call does not handle message fragmentation, use [`WebSocket::read`] instead.
    ///
    /// This call handles ping frame automatically.
    pub async fn read_frame(&mut self) -> io::Result<Frame> {
        if self.closed {
            return Err(io_err!(ConnectionAborted, "connection already closed"));
        }

        loop {
            let frame = poll_fn(|cx|self.poll_frame(cx)).await?;

            match frame.opcode() {
                OpCode::Close => {
                    self.closed = true;
                },
                OpCode::Ping => {
                    self.send_pong(&frame.into_payload()).await?;
                    continue;
                },
                OpCode::Text => {},
                OpCode::Binary => {}
                OpCode::Continuation => {}
                OpCode::Pong => {}
            }

            return Ok(frame)
        }
    }

    fn poll_message(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<Frame>> {
        loop {
            let frame = ready!(self.poll_frame(cx)?);
            let frags = self.fragment.take();

            if frags.is_some() && frame.opcode() != OpCode::Continuation {
                return Poll::Ready(Err(io_err!("fragment frame did not followed by continuation frame")))
            }

            match (frags, frame.fin()) {
                (None, true) => {
                    return Poll::Ready(Ok(frame))
                },
                (None, false) => {
                    self.fragment = Some(frame);
                },
                (Some(mut frags), true) => {
                    frags.payload_mut().unsplit(frame.into_payload());
                    return Poll::Ready(Ok(frags))
                },
                (Some(mut frags), false) => {
                    frags.payload_mut().unsplit(frame.into_payload());
                    self.fragment = Some(frags);
                },
            }

        }
    }

    fn poll_frame(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<Frame>> {

        // ===== Header =====
        // https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API/Writing_WebSocket_servers#exchanging_data_frames

        let Some(headline) = self.read_buf.get(..2) else {
            self.read_buf.reserve(2);
            ready!(poll_read(&mut self.io, &mut self.read_buf, cx)?);
            return self.poll_frame(cx)
        };

        let fin  = headline[0] & 0b10000000 != 0;
        let rsv1 = headline[0] & 0b01000000 != 0;
        let rsv2 = headline[0] & 0b00100000 != 0;
        let rsv3 = headline[0] & 0b00010000 != 0;
        let opcd = headline[0] & 0b00001111;

        let masked = headline[1] & 0b10000000 != 0;
        let length = headline[1] & 0b01111111; // 0x7F

        if rsv1 || rsv2 || rsv3 {
            return Poll::Ready(Err(io_err!("reserved bits not zero")));
        }

        let Some(opcode) = OpCode::try_from_byte(opcd) else {
            return Poll::Ready(Err(io_err!("unknown opcode")));
        };

        if !masked {
            return Poll::Ready(Err(io_err!("frame is unmasked")));
        }

        if opcode.is_control() && !fin {
            return Poll::Ready(Err(io_err!("control frame cannot be fragmented")));
        }

        // ===== Payload Length =====
        // https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API/Writing_WebSocket_servers#decoding_payload_length

        let extra_payload_size = match length {
            126 => 2,
            127 => 8,
            // <= 125
            _ => 0,
        };
        let payload_len_read = extra_payload_size + MASK_SIZE;

        let Some(mut header) = self.read_buf.get(2..2 + payload_len_read) else {
            self.read_buf.reserve(payload_len_read);
            ready!(poll_read(&mut self.io, &mut self.read_buf, cx)?);
            return self.poll_frame(cx)
        };

        let payload_len: usize = match extra_payload_size {
            0 => usize::from(length),
            2 => header.get_u16() as usize,

            #[cfg(target_pointer_width = "64")]
            8 => header.get_u64() as usize,
            #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
            8 => match usize::try_from(header.get_u64()) {
                Ok(length) => length,
                Err(_) => return Err(io_err!("frame too large")),
            },
            _ => unreachable!(),
        };

        debug_assert!(masked);
        let mask_key = header.get_u32().to_be_bytes();

        // for pings and pongs, the max payload length is 125
        if opcode == OpCode::Ping && payload_len > 125 {
            return Poll::Ready(Err(io_err!("ping frame too large")));
        }

        if payload_len >= MAX_FRAME_SIZE {
            return Poll::Ready(Err(io_err!("frame too large")));
        }

        if self.read_buf[2 + payload_len_read..].len() < payload_len {
            self.read_buf.reserve(payload_len + MAX_HEADER_SIZE);
            ready!(poll_read(&mut self.io, &mut self.read_buf, cx)?);
            return self.poll_frame(cx)
        }

        self.read_buf.advance(2 + payload_len_read);

        let mut payload = self.read_buf.split_to(payload_len);
        mask::unmask(&mut payload, mask_key);

        Poll::Ready(Ok(Frame::new(fin, opcode, payload)))
    }

    /// Send string frame to the client.
    pub async fn send_string(&mut self, string: &str) -> io::Result<()> {
        if self.closed {
            return Err(io_err!(ConnectionAborted, "connection already closed"));
        }
        self.send(true, OpCode::Text, string.as_bytes()).await
    }

    /// Send binary frame to the client.
    pub async fn send_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.closed {
            return Err(io_err!(ConnectionAborted, "connection already closed"));
        }
        self.send(true, OpCode::Binary, bytes).await
    }

    /// Send ping frame to the client.
    pub async fn ping(&mut self, payload: &[u8]) -> io::Result<()> {
        if self.closed {
            return Err(io_err!(ConnectionAborted, "connection already closed"));
        }
        self.send(true, OpCode::Ping, payload).await
    }

    /// Send close frame to the client.
    pub async fn close(mut self) -> io::Result<()> {
        if self.closed {
            return Err(io_err!(ConnectionAborted, "connection already closed"));
        }
        self.send(true, OpCode::Close, &[]).await
    }

    async fn send_pong(&mut self, payload: &[u8]) -> io::Result<()> {
        self.flush().await?;
        self.send(true, OpCode::Pong, payload).await
    }

    fn send<'a>(&'a mut self, fin: bool, opcode: OpCode, payload: &'a [u8]) -> FrameSendFuture<'a> {
        let mut head = [0u8;MAX_HEADER_SIZE - MASK_SIZE];
        head[0] = (fin as u8) << 7 | opcode as u8;

        let len = payload.len();
        let header_size = match len {
            _ if len < 126 => {
                head[1] = len as u8;
                2
            },
            _ if len < 65536 => {
                head[1] = 126;
                head[2..4].copy_from_slice(&(len as u16).to_be_bytes());
                4
            },
            _ => {
                head[1] = 127;
                head[2..10].copy_from_slice(&(len as u64).to_be_bytes());
                10
            }
        };

        FrameSendFuture { head, header_size, payload, io: &mut self.io, phase: Phase::Init }
    }

    pub async fn flush(&mut self) -> io::Result<()> {
        if self.closed {
            return Err(io_err!(ConnectionAborted, "connection already closed"));
        }
        if self.write_buf.is_empty() {
            return Ok(())
        }
        poll_fn(|cx|poll_write_all(&mut self.io, &mut self.write_buf, cx)).await
    }
}

// ===== Utils =====

fn poll_read<R, B>(reader: &mut R, buf: &mut B, cx: &mut std::task::Context) -> Poll<io::Result<usize>>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
    B: bytes::BufMut + ?Sized,
{
    use tokio::io::ReadBuf;

    if !buf.has_remaining_mut() {
        return Poll::Ready(Ok(0));
    }

    let n = {
        let dst = buf.chunk_mut();
        let dst = unsafe { dst.as_uninit_slice_mut() };
        let mut buf = ReadBuf::uninit(dst);
        let ptr = buf.filled().as_ptr();
        ready!(Pin::new(reader).poll_read(cx, &mut buf)?);

        // Ensure the pointer does not change from under us
        assert_eq!(ptr, buf.filled().as_ptr());
        buf.filled().len()
    };

    // SAFETY: This is guaranteed to be the number of initialized (and read)
    // bytes due to the invariants provided by `ReadBuf::filled`.
    unsafe {
        buf.advance_mut(n);
    }

    if n == 0 {
        Poll::Ready(Err(io::Error::new(io::ErrorKind::UnexpectedEof, "unexpected EOF")))
    } else {
        Poll::Ready(Ok(n))
    }
}

fn poll_write_all<W, B>(writer: &mut W, buf: &mut B, cx: &mut std::task::Context) -> Poll<io::Result<()>>
where
    W: tokio::io::AsyncWrite + Unpin + ?Sized,
    B: bytes::Buf + ?Sized,
{
    use std::{io::IoSlice, pin::Pin, task::ready};

    const MAX_VECTOR_ELEMENTS: usize = 64;

    while buf.has_remaining() {
        let n = if writer.is_write_vectored() {
            let mut slices = [IoSlice::new(&[]); MAX_VECTOR_ELEMENTS];
            let cnt = buf.chunks_vectored(&mut slices);
            ready!(Pin::new(&mut *writer).poll_write_vectored(cx, &slices[..cnt]))?
        } else {
            ready!(Pin::new(&mut *writer).poll_write(cx, buf.chunk())?)
        };
        buf.advance(n);
        if n == 0 {
            return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
        }
    }

    Poll::Ready(Ok(()))
}

// ===== Futures =====

#[must_use = "`Future` do nothing unless being polled/awaited"]
pub struct FrameSendFuture<'a> {
    head: [u8; 10],
    header_size: usize,
    payload: &'a [u8],
    io: &'a mut TokioIo<Upgraded>,
    phase: Phase,
}

impl<'a> fmt::Debug for FrameSendFuture<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameSendFuture").finish_non_exhaustive()
    }
}

enum Phase {
    Init,
    Partial(usize),
    Payload,
}

impl<'a> Future for FrameSendFuture<'a> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        use tokio::io::AsyncWrite;
        let me = self.get_mut();
        let total = me.header_size + me.payload.len();

        loop {
            match &mut me.phase {
                Phase::Init => {
                    let io_slice = [IoSlice::new(&me.head[..me.header_size]),IoSlice::new(me.payload)];
                    let write = ready!(Pin::new(&mut *me.io).poll_write_vectored(cx, &io_slice)?);
                    if write == total {
                        return Poll::Ready(Ok(()))
                    }
                    me.phase = Phase::Partial(write);
                }
                Phase::Partial(write) => {
                    let mut io_slice = [IoSlice::new(&me.head[*write..me.header_size]),IoSlice::new(me.payload)];
                    while *write <= me.header_size {
                        io_slice[0] = IoSlice::new(&me.head[*write..me.header_size]);
                        *write += ready!(Pin::new(&mut *me.io).poll_write_vectored(cx, &io_slice)?);
                    }
                    if *write == total {
                        return Poll::Ready(Ok(()));
                    }
                    me.payload = &me.payload[*write..];
                    me.phase = Phase::Payload;
                }
                Phase::Payload => {
                    if !me.payload.is_empty() {
                        ready!(poll_write_all(me.io, &mut me.payload, cx)?);
                    }
                    return Poll::Ready(Ok(()))
                }
            }
        }
    }
}

// ===== Masking =====

mod mask {
    #[inline]
    fn unmask_fallback(buf: &mut [u8], mask: [u8; 4]) {
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte ^= mask[i & 3];
        }
    }

    /// https://github.com/snapview/tungstenite-rs/blob/e5efe537b87a6705467043fe44bb220ddf7c1ce8/src/protocol/frame/mask.rs#L23
    #[inline]
    pub fn unmask(buf: &mut [u8], mask: [u8; 4]) {
        let mask_u32 = u32::from_ne_bytes(mask);

        let (prefix, words, suffix) = unsafe { buf.align_to_mut::<u32>() };
        unmask_fallback(prefix, mask);
        let head = prefix.len() & 3;
        let mask_u32 = if head > 0 {
            if cfg!(target_endian = "big") {
                mask_u32.rotate_left(8 * head as u32)
            } else {
                mask_u32.rotate_right(8 * head as u32)
            }
        } else {
            mask_u32
        };
        for word in words.iter_mut() {
            *word ^= mask_u32;
        }
        unmask_fallback(suffix, mask_u32.to_ne_bytes());
    }
}

// ===== Error =====

/// An Error which can occur during http upgrade.
#[derive(Debug)]
pub enum WsUpgradeError {
    /// Header did not represent http upgrade.
    Header(&'static str),
}

impl std::error::Error for WsUpgradeError { }

impl fmt::Display for WsUpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WsUpgradeError::Header(s) => f.write_str(s),
        }
    }
}

impl IntoResponse for WsUpgradeError {
    fn into_response(self) -> Response {
        match self {
            WsUpgradeError::Header(msg) => (StatusCode::BAD_REQUEST,msg).into_response(),
        }
    }
}

impl Stream for WebSocket {
    type Item = io::Result<Frame>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().poll_message(cx).map(Some)
    }
}

// ===== Macros =====

macro_rules! assert_hdr {
    ($h:ident,$id:ident,$target:literal,$err:literal) => {
        match $h.get($id) {
            Some(header) => if header != &$target[..] {
                return ready(Err(WsUpgradeError::Header($err)))
            },
            None => return ready(Err(WsUpgradeError::Header($err)))
        }
    };
    ($h:ident,$id:ident,$target:expr,$err:literal) => {
        match $h.get($id).and_then(|e|e.to_str().ok()) {
            Some($h) => if $target { } else {
                return ready(Err(WsUpgradeError::Header($err)))
            },
            None => return ready(Err(WsUpgradeError::Header($err)))
        }
    };
}

/// `io_err!(ConnectionAborted)`
/// `io_err!(ConnectionAborted, "already closed")`
/// `io_err!("already closed")`
macro_rules! io_err {
    ($kind:ident) => {
        io::Error::from(io::ErrorKind::$kind)
    };
    ($kind:ident,$e:expr) => {
        io::Error::new(io::ErrorKind::$kind, $e)
    };
    ($e:literal) => {
        io::Error::new(io::ErrorKind::InvalidData, $e)
    };
}

pub(super) use {assert_hdr, io_err};

