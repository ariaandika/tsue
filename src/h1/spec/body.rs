use super::ProtoError;
use crate::headers::{
    HeaderMap,
    standard::{CONTENT_LENGTH, TRANSFER_ENCODING},
};

const MAX_CODING: usize = 4;

#[derive(Debug)]
pub struct MessageBody {
    coding: Coding,
}

#[derive(Clone, Debug)]
pub enum Coding {
    None,
    Chunked(Chunked),
    ContentLength(u64),
}

#[derive(Clone, Debug)]
pub struct Chunked {
    slots: [Encoding; MAX_CODING],
    len: u8,
}

/// www.rfc-editor.org/rfc/rfc9110.html#name-content-codings
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Chunked,
    Compress,
    Deflate,
    Gzip,
}

impl Encoding {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buffer = [0u8; 8];
        buffer.get_mut(..bytes.len())?.copy_from_slice(bytes);
        buffer.make_ascii_lowercase();
        match &buffer {
            b"chunked\0" => Some(Self::Chunked),
            b"compress" => Some(Self::Compress),
            b"deflate\0" => Some(Self::Deflate),
            b"gzip\0\0\0\0" => Some(Self::Gzip),
            _ => None
        }
    }
}

impl Chunked {
    const fn new() -> Self {
        Self {
            slots: [Encoding::Chunked; MAX_CODING],
            len: 0,
        }
    }

    fn push(&mut self, encoding: Encoding) -> Option<()> {
        let slot_mut = self.slots.get_mut(self.len as usize)?;
        *slot_mut = encoding;
        self.len += 1;
        Some(())
    }

    fn as_slice(&self) -> &[Encoding] {
        unsafe { std::slice::from_raw_parts(self.slots.as_ptr(), self.len as usize) }
    }
}

impl MessageBody {
    pub fn new(headers: &HeaderMap) -> Result<Self, ProtoError> {
        let mut content_lengths = headers.get_all(CONTENT_LENGTH);
        let transfer_encodings = headers.get_all(TRANSFER_ENCODING);

        let coding = match (content_lengths.next(), transfer_encodings.has_remaining()) {
            (None, false) => Coding::ContentLength(0),
            (None, true) => {
                let mut chunked = Chunked::new();

                for encoding in transfer_encodings
                    .flat_map(|e| e.as_bytes().split(|&e| e == b','))
                    .map(<[u8]>::trim_ascii)
                {
                    let Some(encoding) = Encoding::from_bytes(encoding) else {
                        return Err(ProtoError::InvalidCodings);
                    };
                    let Some(()) = chunked.push(encoding) else {
                        return Err(ProtoError::TooManyEncodings);
                    };
                }

                match chunked.as_slice().last() {
                    Some(encoding) if encoding == &Encoding::Chunked => {}
                    None | Some(_) => return Err(ProtoError::InvalidCodings),
                }

                Coding::Chunked(chunked)
            }
            (Some(length), false) => {
                if content_lengths.has_remaining() {
                    return Err(ProtoError::InvalidContentLength);
                }
                match tcio::atou(length.as_bytes()) {
                    Some(length) => Coding::ContentLength(length),
                    None => return Err(ProtoError::InvalidContentLength),
                }
            }
            (Some(_), true) => return Err(ProtoError::InvalidCodings),
        };
        Ok(Self { coding })
    }
}

