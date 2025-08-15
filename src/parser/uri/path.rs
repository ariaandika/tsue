use tcio::{
    ByteStr,
    bytes::{Bytes},
};

use crate::parser::simd::not_ascii_block;
use super::error::InvalidUri;

#[derive(Debug)]
pub struct Path {
    bytes: ByteStr,
    query: u16,
}

impl Path {
    /// `/`
    pub(crate) const fn slash() -> Path {
        Self {
            bytes: ByteStr::from_static("/"),
            query: 1,
        }
    }

    pub const fn path(&self) -> &str {
        match self.query {
            0 => "/",
            q => self.bytes.as_str().split_at(q as usize).0,
        }
    }

    pub const fn query(&self) -> Option<&str> {
        match self.bytes.as_str().split_at_checked((self.query + 1) as usize) {
            Some((_, q)) if q.is_empty() => None,
            Some((_, query)) => Some(query),
            None => None,
        }
    }

    /// Panic in debug mode if bytes is empty.
    pub(crate) fn parse(mut bytes: Bytes) -> Result<Self, InvalidUri> {
        const CHUNK_SIZE: usize = size_of::<usize>();
        const MSB: usize = usize::from_ne_bytes([128; CHUNK_SIZE]);
        const LSB: usize = usize::from_ne_bytes([1; CHUNK_SIZE]);

        const QS: usize = usize::from_ne_bytes([b'?'; CHUNK_SIZE]);
        const HASH: usize = usize::from_ne_bytes([b'#'; CHUNK_SIZE]);

        debug_assert!(!bytes.is_empty());

        let mut cursor = bytes.cursor_mut();

        'swar: {
            while let Some(chunk) = cursor.peek_chunk::<CHUNK_SIZE>() {
                let value = usize::from_ne_bytes(*chunk);

                // look for "?"
                let qs_xor = value ^ QS;
                let qs_result = qs_xor.wrapping_sub(LSB) & !qs_xor;

                // look for "#"
                let hash_xor = value ^ HASH;
                let hash_result = hash_xor.wrapping_sub(LSB) & !hash_xor;

                let result = (qs_result | hash_result) & MSB;
                if result != 0 {
                    cursor.advance((result.trailing_zeros() / 8) as usize);
                    break 'swar;
                }

                // validate ASCII (< 127)
                if not_ascii_block!(value) {
                    return Err(InvalidUri::NonAscii);
                }

                cursor.advance(CHUNK_SIZE);
            }

            while let Some(b) = cursor.next() {
                if matches!(b, b'?' | b'#') {
                    cursor.step_back(1);
                    break 'swar;
                } else if !b.is_ascii() {
                    return Err(InvalidUri::NonAscii);
                }
            }

            // contained full path
        };

        let (query, path) = match cursor.peek() {
            Some(b'#') => {
                cursor.truncate_buf();
                (bytes.len(), bytes)
            }
            Some(b'?') => {
                let steps = cursor.steps();

                'swar: {
                    while let Some(chunk) = cursor.peek_chunk::<CHUNK_SIZE>() {
                        let value = usize::from_ne_bytes(*chunk);

                        // look for "#"
                        let hash_xor = value ^ HASH;
                        let hash_result = hash_xor.wrapping_sub(LSB) & !hash_xor & MSB;

                        if hash_result != 0 {
                            cursor.advance((hash_result.trailing_zeros() / 8) as usize);
                            cursor.truncate_buf();
                            break 'swar;
                        }

                        // validate ASCII (< 127)
                        if not_ascii_block!(value) {
                            return Err(InvalidUri::NonAscii);
                        }

                        cursor.advance(CHUNK_SIZE);
                    }

                    while let Some(b) = cursor.next() {
                        if b == b'#' {
                            cursor.step_back(1);
                            cursor.truncate_buf();
                            break 'swar;
                        } else if !b.is_ascii() {
                            return Err(InvalidUri::NonAscii);
                        }
                    }
                }

                (steps, bytes)
            }
            Some(_) => unreachable!("error in swar lookup"),
            None => (bytes.len(), bytes),
        };

        match u16::try_from(query) {
            Ok(query) => Ok(Self {
                // SAFETY: iterated and validated that all contains ASCII
                bytes: unsafe { ByteStr::from_utf8_unchecked(path) },
                query
            }),
            Err(_) => Err(InvalidUri::TooLong),
        }
    }
}

