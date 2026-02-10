use std::task::Poll;
use tcio::bytes::Buf;
use tcio::bytes::BytesMut;

use crate::h2::frame;
use crate::h2::hpack;
use crate::h2::hpack::Decoder;
use crate::h2::settings::{SettingId, Settings};
use crate::headers::HeaderMap;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const PREFACE: &[u8; 24] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

#[derive(Debug)]
pub struct H2State {
    settings: Settings,
    decoder: hpack::Decoder,
}

impl H2State {
    pub(crate) fn preface_chunk(read_buffer: &mut BytesMut) -> Poll<Result<Self, BoxError>> {
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
        let decoder = Decoder::with_capacity(settings.header_table_size as usize, 16);

        Poll::Ready(Ok(Self { settings, decoder }))
    }
}

impl H2State {
    pub(crate) fn poll_frame(
        &mut self,
        read_buffer: &mut BytesMut,
        write_buffer: &mut BytesMut,
    ) -> Result<Option<()>, BoxError> {
        let Some(frame) = read_buffer
            .first_chunk()
            .copied()
            .map(frame::FrameHeader::decode)
        else {
            return Ok(None);
        };

        let Some(payload) = read_buffer.try_split_to(frame.len()) else {
            return Ok(None);
        };

        let Some(ty) = frame::Type::from_u8(frame.ty) else {
            return Err(format!("unknown frame: {:?}", frame.ty).into());
        };

        use frame::Type as Ty;
        match ty {
            Ty::Headers => {
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
                let mut block = payload.freeze();

                self.decoder.decode_size_update(&mut block)?;

                while !block.is_empty() {
                    let field = self.decoder.decode(&mut block, write_buffer).unwrap();
                    println!("  {field:?}");
                    headers.try_append_field(field.into_owned()).unwrap();
                }
            }
            _ => {
                println!("[{ty:?}] unhandled frame");
            }
        }

        Ok(Some(()))
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

