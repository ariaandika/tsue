use std::task::Poll;
use tcio::bytes::Buf;
use tcio::bytes::BytesMut;

use crate::h2::error::{ConnectionError, HandshakeError};
use crate::h2::frame;
use crate::h2::hpack::Decoder;
use crate::h2::settings::{self, Settings};
use crate::h2::stream::{self, StreamList};
use crate::headers::HeaderMap;

const PREFACE: &[u8; 24] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

#[derive(Debug)]
pub struct H2State {
    #[allow(unused, reason = "TODO")]
    settings: Settings,
    decoder: Decoder,
    streams: StreamList,
}

pub(crate) enum FrameResult {
    None,
    Request(u32, HeaderMap),
    Data(u32, BytesMut),
    Shutdown,
}

impl Default for H2State {
    fn default() -> Self {
        Self::new()
    }
}

impl H2State {
    pub fn new() -> Self {
        let settings = Settings::new();
        Self {
            decoder: Decoder::with_capacity(settings.header_table_size as usize, 16),
            streams: StreamList::new(settings.max_concurrent_streams as usize),
            settings,
        }
    }

    pub(crate) fn handshake(
        read_buffer: &mut BytesMut,
        write_buffer: &mut BytesMut,
    ) -> Poll<Result<(), HandshakeError>> {
        let Some(chunk) = read_buffer.first_chunk::<{ PREFACE.len() + frame::Header::SIZE }>()
        else {
            return Poll::Pending;
        };
        let (preface, frame) = split_exact(chunk);

        if preface != PREFACE {
            return Poll::Ready(Err(HandshakeError::InvalidPreface));
        }
        let Some(frame::Type::Settings) = frame::Header::frame_type_of(frame) else {
            return Poll::Ready(Err(HandshakeError::ExpectedSettings));
        };
        // write_buffer.extend_from_slice(PREFACE);
        write_buffer.extend_from_slice(&frame::Header::EMPTY_SETTINGS);
        read_buffer.advance(PREFACE.len());
        println!("[HANDSHAKE] ok");
        Poll::Ready(Ok(()))
    }

    pub fn streams_mut(&mut self) -> &mut StreamList {
        &mut self.streams
    }
}

const MAX_FRAME_SIZE: usize = 16_384;

impl H2State {
    pub(crate) fn poll_frame(
        &mut self,
        read_buffer: &mut BytesMut,
        write_buffer: &mut BytesMut,
    ) -> Result<Option<FrameResult>, ConnectionError> {
        use ConnectionError as E;

        let Some(frame) = read_buffer.first_chunk() else {
            return Ok(None);
        };
        let frame = frame::Header::decode(frame);
        if read_buffer.len() < frame.frame_size() {
            return Ok(None);
        }
        if frame.len() > MAX_FRAME_SIZE {
            return Err(E::ExcessiveFrame);
        }
        let Some(ty) = frame.frame_type() else {
            return Err(E::UnexpectedFrame);
        };

        use frame::Type as Ty;
        match ty {
            Ty::Headers => {
                // also get the CONINUATION frames if any
                let payload = {
                    let mut continu = !frame.is_end_headers();
                    let mut bytes = &read_buffer[frame.frame_size()..];
                    while continu {
                        let Some(frame) = bytes.first_chunk() else {
                            // CONTINUATION frame have not been read yet
                            return Ok(None);
                        };
                        let frame = frame::Header::decode(frame);
                        let Some(frame::Type::Continuation) = frame.frame_type() else {
                            return Err(E::UnexpectedFrame);
                        };
                        let Some(rest) = bytes.get(frame.frame_size()..) else {
                            return Ok(None);
                        };
                        continu = !frame.is_end_headers();
                        bytes = rest;
                    }
                    let len = bytes.as_ptr().addr() - read_buffer.as_ptr().addr();
                    read_buffer.split_to(len).freeze()
                };

                // get the stream
                if frame.stream_id == 0 {
                    return Err(E::InvalidStreamId);
                }
                let stream = match self.streams.stream_mut(frame.stream_id) {
                    Some(stream) => {
                        // stream that idle will be `None`, and other state which can receive
                        // headers is reserved
                        if !stream.is_reserved() {
                            return Err(E::UnexpectedFrame);
                        }
                        stream
                    },
                    None => {
                        if frame.stream_id & 1 == 0 {
                            return Err(E::InvalidStreamId);
                        }
                        self.streams.create(frame.stream_id)
                    },
                };
                if frame.is_end_stream() {
                    stream.set_state(stream::State::HalfClosedRemote);
                }

                let mut payload = payload;
                let mut headers = HeaderMap::new();

                while let Some(frame) = payload.try_get_chunk() {
                    let frame = frame::Header::decode(&frame);
                    let mut block = payload.split_to(frame.len());

                    println!(
                        "[HEADER] padded={}, end_headers={}, end_stream={}",
                        frame.is_padded(),
                        frame.is_end_headers(),
                        frame.is_end_stream(),
                    );

                    self.decoder.decode_size_update(&mut block)?;

                    while !block.is_empty() {
                        let field = self.decoder.decode(&mut block, write_buffer)?;
                        println!("  {field:?}");
                        if headers.try_append_field(field.into_owned()).is_err() {
                            return Err(E::ExcessiveHeaders);
                        }
                    }
                }

                debug_assert!(payload.is_empty());

                Ok(Some(FrameResult::Request(frame.stream_id, headers)))
            }
            Ty::Data => {
                // get the stream
                if frame.stream_id == 0 {
                    return Err(E::InvalidStreamId);
                }
                match self.streams.stream_mut(frame.stream_id) {
                    Some(stream) => {
                        if let stream::State::Open | stream::State::HalfClosedRemote = stream.state() {
                            return Err(E::UnexpectedFrame);
                        }
                        if frame.is_end_stream() {
                            stream.set_state(stream::State::HalfClosedRemote);
                        }
                    },
                    None => return Err(E::UnexpectedFrame),
                }

                read_buffer.advance(frame::Header::SIZE);
                let data = read_buffer.split_to(frame.len());

                Ok(Some(FrameResult::Data(frame.stream_id, data)))
            }
            Ty::RstStream => {
                // get the stream
                if frame.stream_id == 0 {
                    return Err(E::InvalidStreamId);
                }
                match self.streams.stream_mut(frame.stream_id) {
                    Some(stream) => stream.set_state(stream::State::Closed),
                    None => return Err(E::UnexpectedFrame),
                }

                Ok(Some(FrameResult::None))
            }
            Ty::Settings => {

                if !frame.is_ack() {
                    let mut payload = &read_buffer[
                        frame::Header::SIZE..frame::Header::SIZE + frame.len()
                    ];

                    while let Some((chunk, rest)) = payload.split_first_chunk() {
                        let (id, val) = split_exact::<{ size_of::<u16>() + size_of::<u32>() }, _, _>(chunk);
                        let id = u16::from_be_bytes(*id);
                        let val = u32::from_be_bytes(*val);

                        let Some(id) = settings::Id::from_u16(id) else {
                            return Err(E::UnknownSetting);
                        };

                        println!("[SETTINGS] {id:?} = {val}");
                        self.settings.set_by_id(id, val);
                        payload = rest;
                    }
                }

                write_buffer.extend_from_slice(&frame::Header::ACK_SETTINGS);
                read_buffer.advance(frame.frame_size());
                Ok(Some(FrameResult::None))
            },
            Ty::Ping => {
                if frame.stream_id != 0 {
                    return Err(E::InvalidStreamId);
                }
                if !frame.is_ack() {
                    const EMPTY_OPAQUE_DATA: [u8; 8] = [0; 8];
                    write_buffer.extend_from_slice(&frame::Header::ACK_PING);
                    write_buffer.extend_from_slice(&EMPTY_OPAQUE_DATA);
                }
                read_buffer.advance(frame.frame_size());
                Ok(Some(FrameResult::None))
            }
            Ty::WindowUpdate => {
                read_buffer.advance(frame.frame_size());
                Ok(Some(FrameResult::None))
            }
            Ty::GoAway => {
                read_buffer.advance(frame.frame_size());
                Ok(Some(FrameResult::Shutdown))
            }
            Ty::Priority => {
                println!("[PRIORITY] priority frame are not supported");
                read_buffer.advance(frame.frame_size());
                Ok(Some(FrameResult::None))
            }
            Ty::Continuation | Ty::PushPromise => {
                // CONTINUATION is handled at once in HEADERS branch
                Err(E::UnexpectedFrame)
            }
        }
    }
}

fn split_exact<const S: usize, const M: usize, const N: usize>(bytes: &[u8; S]) -> (&[u8; M], &[u8; N]) {
    assert_eq!(M + N, S);
    let chunk1 = bytes[..M].try_into().expect("known size");
    let chunk2 = bytes[M..M + N].try_into().expect("known size");
    (chunk1, chunk2)
}
