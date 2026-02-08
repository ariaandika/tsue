use std::{pin::Pin, task::ready};
use std::task::Poll;
use tcio::bytes::{Buf, BytesMut};
use tcio::io::{AsyncRead, AsyncWrite};

use crate::h2::settings::{SettingId, Settings};
use crate::h2::{frame, hpack};
use crate::headers::HeaderMap;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const PREFACE: &[u8; 24] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
// const MAX_BUFFER_CAP: usize = 16 * 1024;
const DEFAULT_BUFFER_CAP: usize = 512;

macro_rules! io_read {
    ($read:expr) => {
        let read = ready!($read)?;
        if read == 0 {
            return Poll::Ready(Ok(()));
        }
        // if $buffer.len() > MAX_BUFFER_CAP {
        //     return Poll::Ready(Err("excessive field size".into()));
        // }
    };
}

/// HTTP/2.0 Connection.
#[derive(Debug)]
pub struct Connection<IO> {
    io: IO,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    /// will be `None` pre-preface
    settings: Option<Settings>,
    decoder: hpack::Decoder,
}

type ConnectionProject<'a, IO> = (
    Pin<&'a mut IO>,
    &'a mut BytesMut,
    &'a mut BytesMut,
    &'a mut Option<Settings>,
    &'a mut hpack::Decoder,
);

impl<IO> Connection<IO> {
    pub fn new(io: IO) -> Self {
        Self {
            io,
            read_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            write_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            settings: None,
            decoder: hpack::Decoder::default(),
        }
    }
}

impl<IO> Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{

    fn try_poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Result<(), BoxError>> {
        let (mut io, read_buffer, write_buffer, settings, hpack) = self.as_mut().project();

        let _settings = match settings.as_mut() {
            Some(ok) => ok,
            None => match Self::preface(read_buffer, settings) {
                Poll::Ready(result) => result?,
                Poll::Pending => {
                    io_read!(io.as_mut().poll_read(read_buffer, cx));
                    return self.try_poll(cx);
                }
            }
        };

        while let Some(header) = read_buffer.first_chunk() {
            let frame = frame::FrameHeader::decode(*header);
            read_buffer.advance(header.len());

            let Some(payload) = read_buffer.try_split_to(frame.len()) else {
                break;
            };

            let Some(ty) = frame::Type::from_u8(frame.ty) else {
                println!("[ERROR] unknown frame: {:?}", frame.ty);
                return Poll::Ready(Err("unknown frame".into()));
            };

            use frame::Type as F;
            match ty {
                F::Headers => {
                    const PRIORITY_MASK: u8 = 0x20;
                    const PADDED_MASK: u8 = 0x08;
                    const END_HEADERS_MASK: u8 = 0x04;
                    const END_STREAM_MASK: u8 = 0x01;

                    println!(
                        "[HEADER] priority={}, padded={}, end_headers={}, end_stream={}",
                        frame.flags & PRIORITY_MASK != 0,
                        frame.flags & PADDED_MASK != 0,
                        frame.flags & END_HEADERS_MASK != 0,
                        frame.flags & END_STREAM_MASK != 0,
                    );

                    let mut headers = HeaderMap::new();
                    let mut decoder = hpack.decode_block(payload.freeze(), write_buffer);
                    while let Some(field) = decoder.next_field()? {
                        println!("  {field:?}");
                        headers.try_append_field(field)?;
                    }
                }
                F::WindowUpdate => {
                    let size = u32::from_be_bytes(*payload.first_chunk().expect("TODO"));
                    println!("[{ty:?}] window size increment = {size}");

                    const _SIZE: usize = 1_048_576_000;
                    // let mut window_size = 65535;
                    // window_size += u32::from_be_bytes(*size);
                }
                _ => {
                    println!("[{ty:?}] unhandled frame");
                }
            };
        }

        if ready!(io.poll_read(&mut *read_buffer, cx)?) == 0 {
            return Poll::Ready(Ok(()))
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }

    fn preface<'a>(
        read_buffer: &mut BytesMut,
        settings_mut: &'a mut Option<Settings>,
    ) -> Poll<Result<&'a mut Settings, BoxError>> {
        let Some((preface, header, rest)) = split_exact(read_buffer) else {
            return Poll::Pending;
        };

        if preface != *PREFACE {
            return Poll::Ready(Err("preface error".into()));
        }

        let frame = frame::FrameHeader::decode(header);
        let mut settings = Settings::new();

        if !matches!(frame.frame_type(), Some(frame::Type::Settings)) {
            return Poll::Ready(Err("malformed frame".into()));
        }

        let total_len = PREFACE.len() + frame::FrameHeader::SIZE + frame.len();

        let Some(mut payload) = rest.get(..frame.len()) else {
            return Poll::Pending;
        };

        while let Some((id, val, rest)) = split_exact(payload) {
            let id = u16::from_be_bytes(id);
            let val = u32::from_be_bytes(val);

            let Some(id) = SettingId::from_u16(id) else {
                return Poll::Ready(Err("invalid setting id".into()));
            };

            println!("[SETTINGS] {id:?} = {val}");
            settings.set_by_id(id, val);
            payload = rest;
        }

        read_buffer.advance(total_len);

        Poll::Ready(Ok(settings_mut.get_or_insert(settings)))
    }
}

impl<IO> Future for Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        if let Err(err) = ready!(self.try_poll(cx)) {
            eprintln!("{err}")
        }
        Poll::Ready(())
    }
}

fn split_exact<const M: usize, const N: usize>(bytes: &[u8]) -> Option<([u8; M], [u8; N], &[u8])> {
    if bytes.len() < M + N {
        return None;
    }
    let chunk1 = bytes[..M].try_into().expect("known size");
    let chunk2 = bytes[M..M + N].try_into().expect("known size");
    Some((chunk1, chunk2, &bytes[M + N..]))
}

// ===== Projection =====

impl<IO> Connection<IO> {
    fn project(self: Pin<&mut Self>) -> ConnectionProject<'_, IO> {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe {
            let me = self.get_unchecked_mut();
            (
                Pin::new_unchecked(&mut me.io),
                &mut me.read_buffer,
                &mut me.write_buffer,
                &mut me.settings,
                &mut me.decoder,
            )
        }
    }
}

