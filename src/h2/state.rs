use std::task::Poll;
use tcio::bytes::Buf;
use tcio::bytes::BytesMut;

use crate::h2::frame;
use crate::h2::hpack::Decoder;
use crate::h2::settings::{self, Settings};
use crate::h2::stream;
use crate::h2::stream::StreamList;
use crate::headers::HeaderMap;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

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

impl H2State {
    pub(crate) fn handshake(
        read_buffer: &mut BytesMut,
        write_buffer: &mut BytesMut,
    ) -> Poll<Result<Self, BoxError>> {
        let Some((preface, header, rest)) = split_exact(read_buffer) else {
            return Poll::Pending;
        };

        if preface != *PREFACE {
            return Poll::Ready(Err("invalid preface".into()));
        }
        let frame = frame::Header::decode(header);

        let Some(mut payload) = rest.get(..frame.len()) else {
            return Poll::Pending;
        };

        if !matches!(frame.frame_type(), Some(frame::Type::Settings)) {
            return Poll::Ready(Err("expected settings frame".into()));
        }

        let mut settings = Settings::new();

        while let Some((id, val, rest)) = split_exact(payload) {
            let id = u16::from_be_bytes(id);
            let val = u32::from_be_bytes(val);

            let Some(id) = settings::Id::from_u16(id) else {
                return Poll::Ready(Err("invalid setting id".into()));
            };

            println!("[SETTINGS] {id:?} = {val}");
            settings.set_by_id(id, val);
            payload = rest;
        }

        let total_len = PREFACE.len() + frame.frame_size();
        read_buffer.advance(total_len);

        // write_buffer.extend_from_slice(PREFACE);
        write_buffer.extend_from_slice(&frame::Header::EMPTY_SETTINGS);
        write_buffer.extend_from_slice(&frame::Header::ACK_SETTINGS);

        let decoder = Decoder::with_capacity(settings.header_table_size as usize, 16);
        let streams = StreamList::new(settings.max_concurrent_streams as usize);

        Poll::Ready(Ok(Self { settings, decoder, streams, }))
    }

    pub fn streams_mut(&mut self) -> &mut StreamList {
        &mut self.streams
    }
}

impl H2State {
    pub(crate) fn poll_frame(
        &mut self,
        read_buffer: &mut BytesMut,
        write_buffer: &mut BytesMut,
    ) -> Result<Option<FrameResult>, BoxError> {
        let Some(frame) = read_buffer.first_chunk() else {
            return Ok(None);
        };
        let frame = frame::Header::decode(*frame);
        if read_buffer.len() < frame.frame_size() {
            return Ok(None);
        }
        let Some(ty) = frame.frame_type() else {
            return Err(format!("unknown frame: {:?}", frame.ty).into());
        };

        use frame::Type as Ty;
        match ty {
            Ty::Headers => {
                const PRIORITY: u8 = 0x20;
                const PADDED: u8 = 0x08;
                const END_HEADERS: u8 = 0x04;
                const END_STREAM: u8 = 0x01;

                // CONINUATION validation
                // guarantee that all wanted frames is in buffer
                if frame.flags & END_HEADERS != END_HEADERS {
                    let mut bytes = &read_buffer[frame.frame_size()..];
                    loop {
                        let Some(frame) = bytes.first_chunk() else {
                            // CONTINUATION frame have not been read yet
                            return Ok(None);
                        };
                        let frame = frame::Header::decode(*frame);
                        let Some(frame::Type::Continuation) = frame.frame_type() else {
                            return Err("expected CONTINUATION frame".into());
                        };
                        if frame.flags & END_HEADERS == END_HEADERS {
                            break
                        }
                        let Some(rest) = bytes.get(frame.frame_size()..) else {
                            return Ok(None);
                        };
                        bytes = rest;
                    }
                }

                // get the stream
                if frame.stream_id == 0 {
                    return Err("stream id 0 in HEADERS frame".into());
                }
                let stream = match self.streams.stream_mut(frame.stream_id) {
                    Some(stream) => {
                        // stream that idle will be `None`, and other state which can receive
                        // headers is reserved
                        if !stream.is_reserved() {
                            return Err(format!("unexpected headers frame for stream({})",frame.stream_id).into());
                        }
                        stream
                    },
                    None => {
                        if frame.stream_id & 1 == 0 {
                            return Err("even stream id from client".into());
                        }
                        self.streams.create(frame.stream_id)
                    },
                };
                if frame.flags & END_STREAM == END_STREAM {
                    stream.set_state(stream::State::HalfClosedRemote);
                }

                println!(
                    "[HEADER] priority={}, padded={}, end_headers={}, end_stream={}",
                    frame.flags & PRIORITY != 0,
                    frame.flags & PADDED != 0,
                    frame.flags & END_HEADERS != 0,
                    frame.flags & END_STREAM != 0,
                );

                read_buffer.advance(frame::Header::SIZE);
                let mut headers = HeaderMap::new();

                // HEADERS frame
                {
                    let mut block = read_buffer.try_split_to(frame.len()).expect("validated").freeze();
                    self.decoder.decode_size_update(&mut block)?;
                    while !block.is_empty() {
                        let field = self.decoder.decode(&mut block, write_buffer).unwrap();
                        println!("  {field:?}");
                        headers.try_append_field(field.into_owned()).unwrap();
                    }
                }

                // CONTINUATION frame
                if frame.flags & END_HEADERS != END_HEADERS {
                    let frame = frame::Header::decode(
                        read_buffer
                            .try_get_chunk()
                            .expect("validated, TODO: use unsafe to skip bounds check"),
                    );
                    let mut block = read_buffer
                        .try_split_to(frame.len())
                        .expect("validated")
                        .freeze();

                    println!(
                        "[CONTINUATION] priority={}, padded={}, end_headers={}, end_stream={}",
                        frame.flags & PRIORITY != 0,
                        frame.flags & PADDED != 0,
                        frame.flags & END_HEADERS != 0,
                        frame.flags & END_STREAM != 0,
                    );

                    self.decoder.decode_size_update(&mut block)?;
                    while !block.is_empty() {
                        let field = self.decoder.decode(&mut block, write_buffer).unwrap();
                        println!("  {field:?}");
                        headers.try_append_field(field.into_owned()).unwrap();
                    }
                }

                Ok(Some(FrameResult::Request(frame.stream_id, headers)))
            }
            Ty::Data => {
                // const PADDED: u8 = 0x08;
                const END_STREAM: u8 = 0x01;

                // get the stream
                if frame.stream_id == 0 {
                    return Err("stream id 0 in DATA frame".into());
                }
                match self.streams.stream_mut(frame.stream_id) {
                    Some(stream) => {
                        if let stream::State::Open | stream::State::HalfClosedRemote = stream.state() {
                            return Err(format!("unexpected headers frame for stream({})",frame.stream_id).into());
                        }
                        if frame.flags & END_STREAM == END_STREAM {
                            stream.set_state(stream::State::HalfClosedRemote);
                        }
                    },
                    None => return Err("stream is idle".into()),
                }

                read_buffer.advance(frame::Header::SIZE);
                let data = read_buffer.split_to(frame.len());

                Ok(Some(FrameResult::Data(frame.stream_id, data)))
            }
            Ty::RstStream => {
                // get the stream
                if frame.stream_id == 0 {
                    return Err("stream id 0 in RST_STREAM frame".into());
                }
                match self.streams.stream_mut(frame.stream_id) {
                    Some(stream) => stream.set_state(stream::State::Closed),
                    None => return Err("stream is idle".into()),
                }

                Ok(Some(FrameResult::None))
            }
            Ty::Ping => {
                const ACK: u8 = 0x01;

                if frame.stream_id != 0 {
                    return Err("non zero stream id in PING frame".into());
                }
                if frame.flags & ACK != ACK {
                    const EMPTY_OPAQUE_DATA: [u8; 8] = [0; 8];
                    write_buffer.extend_from_slice(&frame::Header::ACK_PING);
                    write_buffer.extend_from_slice(&EMPTY_OPAQUE_DATA);
                }
                read_buffer.advance(frame.frame_size());

                Ok(Some(FrameResult::None))
            }
            Ty::Settings => todo!(),
            Ty::WindowUpdate => todo!(),
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
                Err("unexpected frame from client".into())
            }
        }
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

