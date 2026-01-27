use std::{pin::Pin, task::ready};
use std::task::Poll;
use tcio::bytes::{Buf, BytesMut};
use tcio::io::{AsyncRead, AsyncWrite};

use crate::h2::settings::{SettingId, Settings, SettingsError};
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
    phase: Phase,
    settings: Settings,
    hpack: hpack::Table,
}

type ConnectionProject<'a, IO> = (
    Pin<&'a mut IO>,
    &'a mut BytesMut,
    &'a mut BytesMut,
    Pin<&'a mut Phase>,
    &'a mut Settings,
    &'a mut hpack::Table,
);

#[derive(Debug)]
enum Phase {
    Preface,
    Idle,
}

enum PhaseProject {
    Preface,
    Idle,
}

impl<IO> Connection<IO> {
    pub fn new(io: IO) -> Self {
        Self {
            io,
            read_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            write_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            phase: Phase::Preface,
            settings: Settings::new(),
            hpack: hpack::Table::default(),
        }
    }

    fn try_poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Result<(), BoxError>>
    where
        IO: AsyncRead + AsyncWrite,
    {
        let (mut io, read_buffer, write_buffer, mut phase, settings, hpack) = self.project();

        loop {
            match phase.as_mut().project() {
                PhaseProject::Preface => {
                    let Some(preface) = read_buffer.first_chunk() else {
                        io_read!(io.as_mut().poll_read(&mut *read_buffer, cx));
                        continue;
                    };

                    if preface != PREFACE {
                        return Poll::Ready(Err("preface error".into()));
                    }

                    read_buffer.advance(PREFACE.len());
                    phase.set(Phase::Idle);
                }
                PhaseProject::Idle => {
                    'idle: loop {
                        let frame;
                        let payload;

                        'lead: {
                            if let Some((frame_buf, rest)) = read_buffer.split_first_chunk() {
                                frame = frame::FrameHeader::decode(*frame_buf);
                                if frame.len() <= rest.len() {
                                    read_buffer.advance(frame_buf.len());
                                    payload = read_buffer.split_to(frame.len());
                                    break 'lead
                                }
                            };

                            if ready!(io.as_mut().poll_read(&mut *read_buffer, cx)?) == 0 {
                                return Poll::Ready(Ok(()))
                            }
                            continue 'idle;
                        };

                        let Some(ty) = frame::Type::from_u8(frame.ty) else {
                            println!("[ERROR] unknown frame: {:?}", frame.ty);
                            break;
                        };

                        use frame::Type as F;
                        match ty {
                            F::Headers => {
                                const PRIORITY_MASK: u8 = 0x20;
                                const PADDED_MASK: u8 = 0x08;
                                const END_HEADERS_MASK: u8 = 0x04;
                                const END_STREAM_MASK: u8 = 0x01;

                                println!("[{ty:?}] is priority = {}",frame.flags & PRIORITY_MASK != 0);
                                println!("[{ty:?}] is padded = {}",frame.flags & PADDED_MASK != 0);
                                println!("[{ty:?}] is end headers = {}",frame.flags & END_HEADERS_MASK != 0);
                                println!("[{ty:?}] is end stream = {}",frame.flags & END_STREAM_MASK != 0);

                                let mut headers = HeaderMap::new();
                                hpack.decode_block(payload.freeze(), &mut headers, &mut *write_buffer)?;
                                headers.iter().for_each(|e|println!("{e:?}"));

                            }
                            F::Settings => {
                                let mut payload = payload;

                                while let Some((ident, rest)) = payload.split_first_chunk() {
                                    let Ok(value): Result<&[u8; 4], _> = rest.try_into() else {
                                        break;
                                    };
                                    let id = u16::from_be_bytes(*ident);
                                    let val = u32::from_be_bytes(*value);

                                    settings.set_by_id(SettingId::from_u16(id).ok_or(SettingsError::Malformed)?, val);

                                    println!("[SETTING] {id:?} = {val}");
                                    payload.advance(6);
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

                    // dbg!(tcio::fmt::lossy(&buffer));

                    return Poll::Ready(Ok(()));
                }
            }
        }
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
                Pin::new_unchecked(&mut me.phase),
                &mut me.settings,
                &mut me.hpack,
            )
        }
    }
}

impl Phase {
    fn project(self: Pin<&mut Self>) -> PhaseProject {
        // SAFETY: self is pinned, no custom Drop and Unpin
        unsafe {
            match self.get_unchecked_mut() {
                Self::Preface => PhaseProject::Preface,
                Self::Idle => PhaseProject::Idle,
            }
        }
    }
}

