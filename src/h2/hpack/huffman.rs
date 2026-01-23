use tcio::bytes::{BufMut, BytesMut};

use super::huffman_table::DECODE_TABLE;

const DECODED: u8   = 0b001;
const MAYBE_EOS: u8 = 0b010;
const ERROR: u8     = 0b100;

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

#[derive(Debug)]
pub struct HuffmanError;

impl std::error::Error for HuffmanError { }
impl std::fmt::Display for HuffmanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("huffman coding error")
    }
}

#[allow(clippy::unusual_byte_groupings)]
#[test]
fn test_huffman_decode() {
    let mut buf = BytesMut::new();

    // 1100011      H
    // 00101        e
    // 101000       l
    // 00111        o
    // 010100       <SP>
    // 1111000      w
    // 101100       r
    // 100100       d
    // 11111110|00  !
    let bytes = [
        0b1100011_0, // He
        0b0101_1010, // l
        0b00_101000, // l
        0b00111_010, // o<SP>
        0b100_11110, // w
        0b00_00111_1, // or
        0b01100_101, // l
        0b000_10010, // d
        0b0_1111111, // !
        0b000_11111, // <EOS>
    ];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"Hello world!");
}

