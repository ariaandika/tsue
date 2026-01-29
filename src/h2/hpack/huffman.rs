use tcio::bytes::{BufMut, BytesMut};

use super::huffman_table::DECODE_TABLE;
use super::encode_table::ENCODE_TABLE;

const DECODED: u8   = 0b001;
const MAYBE_EOS: u8 = 0b010;
const ERROR: u8     = 0b100;

pub fn encode(bytes: &[u8], buf: &mut BytesMut) {
    let mut tmp = 0u64;
    let mut remaining_bits = 64u8;

    for &byte in bytes {
        let (bits_len, bits) = ENCODE_TABLE[byte as usize];
        let bits = bits as u64;

        match remaining_bits.checked_sub(bits_len) {
            Some(remain) => {
                remaining_bits = remain;
                tmp |= bits << remain;
            }
            None => {
                let overflow = bits_len - remaining_bits;
                remaining_bits = 64 - overflow;

                buf.put_u64(tmp | (bits >> overflow));
                tmp = bits << remaining_bits;
            }
        }
    }

    if remaining_bits >= 64 {
        return;
    }

    let bits_len = 64 - remaining_bits;
    let len = (bits_len / 8) as usize;

    let be_bytes = tmp.to_be_bytes();

    // compiler did not remove bounds checking here, thus unsafe is used

    // SAFETY:
    // - bits_len == 1..64, bits_len / 8 == 0..8
    // - len == 0..8, u64::to_be_bytes() == [u8; 8]
    let filled = unsafe { be_bytes.get_unchecked(..len) };

    buf.extend_from_slice(filled);

    let bits_remain = bits_len % 8;
    if bits_remain != 0 {
        // SAFETY:
        // - bits_len != 64, because 64 % 8 == 0
        // - thus len != 8
        let bits = unsafe { be_bytes.get_unchecked(len) };

        let eos = u8::MAX >> bits_remain;
        buf.put_u8(bits | eos);
    }
}

pub fn decode(bytes: &[u8], buf: &mut BytesMut) -> Result<(), HuffmanError> {
    let mut decoder = Decoder {
        state: 0,
        maybe_eos: true,
    };

    for &byte in bytes {
        if let Some(byte) = decoder.byte(byte >> 4)? {
            buf.put_u8(byte);
        }
        if let Some(byte) = decoder.byte(byte & 0b1111)? {
            buf.put_u8(byte);
        }
    }

    if decoder.maybe_eos || decoder.state == 0 {
        Ok(())
    } else {
        Err(HuffmanError)
    }
}

struct Decoder {
    state: u8,
    maybe_eos: bool,
}

impl Decoder {
    fn byte(&mut self, byte: u8) -> Result<Option<u8>, HuffmanError> {
        let (next, byte, flags) = DECODE_TABLE[self.state as usize][byte as usize];

        if flags & ERROR == ERROR {
            return Err(HuffmanError);
        }

        self.maybe_eos = flags & MAYBE_EOS == MAYBE_EOS;
        self.state = next;

        if flags & DECODED == DECODED {
            Ok(Some(byte))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
pub struct HuffmanError;

impl std::error::Error for HuffmanError { }
impl std::fmt::Display for HuffmanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("huffman coding error")
    }
}

// ===== Test =====

#[cfg(test)]
mod test {
    use super::*;

    // RF7541 Appendix C
    // (raw, encoded)
    const INPUT: &[(&[u8], &[u8])] = &[
        (b"302", &[0x64, 0x02]),
        (b"307", &[0x64, 0x0e, 0xff]),
        (b"no-cache", &[0xa8, 0xeb, 0x10, 0x64, 0x9c, 0xbf]),
        (b"private", &[0xae, 0xc3, 0x77, 0x1a, 0x4b]),
        (b"gzip", &[0x9b, 0xd9, 0xab]),
        (
            b"custom-key",
            &[0x25, 0xa8, 0x49, 0xe9, 0x5b, 0xa9, 0x7d, 0x7f],
        ),
        (
            b"custom-value",
            &[0x25, 0xa8, 0x49, 0xe9, 0x5b, 0xb8, 0xe8, 0xb4, 0xbf],
        ),
        (
            b"www.example.com",
            &[
                0xf1, 0xe3, 0xc2, 0xe5, 0xf2, 0x3a, 0x6b, 0xa0, 0xab, 0x90, 0xf4, 0xff,
            ],
        ),
        (
            b"https://www.example.com",
            &[
                0x9d, 0x29, 0xad, 0x17, 0x18, 0x63, 0xc7, 0x8f, 0x0b, 0x97, 0xc8, 0xe9, 0xae, 0x82,
                0xae, 0x43, 0xd3,
            ],
        ),
        (
            b"Mon, 21 Oct 2013 20:13:21 GMT",
            &[
                0xd0, 0x7a, 0xbe, 0x94, 0x10, 0x54, 0xd4, 0x44, 0xa8, 0x20, 0x05, 0x95, 0x04, 0x0b,
                0x81, 0x66, 0xe0, 0x82, 0xa6, 0x2d, 0x1b, 0xff,
            ],
        ),
        (
            b"foo=ASDJKHQKBZXOQWEOPIUAXQWEOIU; max-age=3600; version=1",
            &[
                0x94, 0xe7, 0x82, 0x1d, 0xd7, 0xf2, 0xe6, 0xc7, 0xb3, 0x35, 0xdf, 0xdf, 0xcd, 0x5b,
                0x39, 0x60, 0xd5, 0xaf, 0x27, 0x08, 0x7f, 0x36, 0x72, 0xc1, 0xab, 0x27, 0x0f, 0xb5,
                0x29, 0x1f, 0x95, 0x87, 0x31, 0x60, 0x65, 0xc0, 0x03, 0xed, 0x4e, 0xe5, 0xb1, 0x06,
                0x3d, 0x50, 0x07,
            ],
        ),
    ];

    #[test]
    fn test_huffman_decode() {
        let mut buf = BytesMut::new();

        for &(raw, encoded) in INPUT {
            decode(encoded, &mut buf).unwrap();
            assert_eq!(
                buf.as_slice(),
                raw,
                "invalid result on {:?}",
                tcio::fmt::lossy(&raw)
            );
            buf.clear();
        }
    }

    #[test]
    fn test_huffman_encode() {
        let mut buf = BytesMut::new();

        for &(raw, encoded) in INPUT {
            encode(raw, &mut buf);
            assert_eq!(
                buf.as_slice(),
                encoded,
                "invalid result on {:?}",
                tcio::fmt::lossy(&raw)
            );
            buf.clear();
        }
    }
}


