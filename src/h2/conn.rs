use std::{pin::Pin, task::ready};
use std::task::Poll;
use tcio::bytes::BytesMut;
use tcio::io::{AsyncIoRead, AsyncIoWrite};

use super::frame;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const PREFACE: &[u8; 24] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const MAX_BUFFER_CAP: usize = 16 * 1024;
const DEFAULT_BUFFER_CAP: usize = 512;

macro_rules! io_read {
    ($io:ident.$read:ident($buffer:ident, $cx:expr)) => {
        let read = ready!($io.$read($buffer, $cx)?);
        if read == 0 {
            return Poll::Ready(Ok(()));
        }
        if $buffer.len() > MAX_BUFFER_CAP {
            return Poll::Ready(Err("excessive field size".into()));
        }
    };
}

/// HTTP/2.0 Connection.
#[derive(Debug)]
pub struct Connection<IO> {
    io: IO,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    phase: Phase,
}

type ConnectionProject<'a, IO> = (
    &'a mut IO,
    &'a mut BytesMut,
    &'a mut BytesMut,
    Pin<&'a mut Phase>,
);

#[derive(Debug)]
enum Phase {
    Preface,
}

enum PhaseProject {
    Preface,
}

impl<IO> Connection<IO> {
    pub fn new(io: IO) -> Self {
        Self {
            io,
            read_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            write_buffer: BytesMut::with_capacity(DEFAULT_BUFFER_CAP),
            phase: Phase::Preface,
        }
    }

    fn try_poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Result<(), BoxError>>
    where
        IO: AsyncIoRead + AsyncIoWrite,
    {
        let (io, read_buffer, _write_buffer, mut phase,) = self.project();

        loop {
            match phase.as_mut().project() {
                PhaseProject::Preface => {
                    let Some((preface, rest)) = read_buffer.split_first_chunk() else {
                        io_read!(io.poll_read_buf(read_buffer, cx));
                        continue;
                    };

                    if preface != PREFACE {
                        return Poll::Ready(Err("preface error".into()));
                    }

                    let mut buffer = rest;

                    loop {
                        let Some((frame, rest)) = buffer.split_first_chunk() else {
                            break;
                        };

                        let frame = frame::Header::decode(*frame);

                        let Some(ty) = frame::Type::from_u8(frame.ty) else {
                            println!("[ERROR] unknown frame: {:?}", frame.ty);
                            break;
                        };

                        println!("[{ty:?}] {frame:?}");

                        use frame::Type as T;
                        let rest = match ty {
                            T::Headers => {
                                const PRIORITY_MASK: u8 = 0x20;
                                const PADDED_MASK: u8 = 0x08;
                                const END_HEADERS_MASK: u8 = 0x04;
                                const END_STREAM_MASK: u8 = 0x01;

                                println!("[{ty:?}] is priority = {}",frame.flags & PRIORITY_MASK != 0);
                                println!("[{ty:?}] is padded = {}",frame.flags & PADDED_MASK != 0);
                                println!("[{ty:?}] is end headers = {}",frame.flags & END_HEADERS_MASK != 0);
                                println!("[{ty:?}] is end stream = {}",frame.flags & END_STREAM_MASK != 0);

                                dbg!(rest.len());
                                dbg!(tcio::fmt::lossy(&rest));
                                &[]
                            }
                            T::Settings => {
                                let Some((mut payload, rest)) = rest.split_at_checked(frame.len()) else {
                                    break;
                                };

                                while let Some((ident, rest)) = payload.split_first_chunk::<2>() {
                                    let Some((value, rest)) = rest.split_first_chunk::<4>() else {
                                        break;
                                    };

                                    let ident = u16::from_be_bytes(*ident);
                                    let value = u32::from_be_bytes(*value);
                                    println!("[{ty:?}] identifier = {ident}");
                                    println!("[{ty:?}] value = {value}");

                                    payload = rest;
                                }

                                // loop {
                                //     let Some((setting, payload)) = payload.split_first_chunk::<2>() else {
                                //         break;
                                //     };
                                //     println!("[{ty:?}] leftover payload = {}",tcio::fmt::lossy(&payload));
                                // }
                                assert!(payload.is_empty());

                                rest
                            }
                            T::WindowUpdate => {
                                let Some((size, rest)) = rest.split_first_chunk::<4>() else {
                                    break;
                                };

                                let size = u32::from_be_bytes(*size);
                                println!("[{ty:?}] window size increment = {size}");

                                const _SIZE: usize = 1_048_576_000;

                                rest
                            }
                            _ => {
                                println!("[{ty:?}] unhandled frame");
                                rest
                            }
                        };

                        buffer = rest;
                    }

                    dbg!(tcio::fmt::lossy(&buffer));

                    return Poll::Ready(Ok(()));
                }
            }
        }
    }
}

impl<IO> Future for Connection<IO>
where
    IO: AsyncIoRead + AsyncIoWrite,
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
                &mut me.io,
                &mut me.read_buffer,
                &mut me.write_buffer,
                Pin::new_unchecked(&mut me.phase),
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
            }
        }
    }
}

