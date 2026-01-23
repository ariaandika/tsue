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

    // RF7541 Appendix C

    let bytes = [
        0xf1, 0xe3, 0xc2, 0xe5, 0xf2, 0x3a, 0x6b, 0xa0,
        0xab, 0x90, 0xf4, 0xff,
    ];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"www.example.com");

    buf.clear();
    let bytes = [0xa8, 0xeb, 0x10, 0x64, 0x9c, 0xbf];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"no-cache");

    buf.clear();
    let bytes = [0x25, 0xa8, 0x49, 0xe9, 0x5b, 0xa9, 0x7d, 0x7f];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"custom-key");

    buf.clear();
    let bytes = [0x25, 0xa8, 0x49, 0xe9, 0x5b, 0xb8, 0xe8, 0xb4, 0xbf];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"custom-value");

    buf.clear();
    let bytes = [0x64, 0x02,];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"302");

    buf.clear();
    let bytes = [0xae, 0xc3, 0x77, 0x1a, 0x4b];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"private");

    buf.clear();
    let bytes = [
        0xd0, 0x7a, 0xbe, 0x94, 0x10, 0x54, 0xd4, 0x44,
        0xa8, 0x20, 0x05, 0x95, 0x04, 0x0b, 0x81, 0x66,
        0xe0, 0x82, 0xa6, 0x2d, 0x1b, 0xff,
    ];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"Mon, 21 Oct 2013 20:13:21 GMT");

    buf.clear();
    let bytes = [
        0x9d, 0x29, 0xad, 0x17, 0x18, 0x63, 0xc7, 0x8f,
        0x0b, 0x97, 0xc8, 0xe9, 0xae, 0x82, 0xae, 0x43,
        0xd3
    ];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"https://www.example.com");

    buf.clear();
    let bytes = [ 0x64, 0x0e, 0xff ];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"307");

    buf.clear();
    let bytes = [0x9b, 0xd9, 0xab];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"gzip");

    buf.clear();
    let bytes = [
        0x94, 0xe7, 0x82, 0x1d, 0xd7, 0xf2, 0xe6, 0xc7,
        0xb3, 0x35, 0xdf, 0xdf, 0xcd, 0x5b, 0x39, 0x60,
        0xd5, 0xaf, 0x27, 0x08, 0x7f, 0x36, 0x72, 0xc1,
        0xab, 0x27, 0x0f, 0xb5, 0x29, 0x1f, 0x95, 0x87,
        0x31, 0x60, 0x65, 0xc0, 0x03, 0xed, 0x4e, 0xe5,
        0xb1, 0x06, 0x3d, 0x50, 0x07
    ];
    decode(&bytes, &mut buf).unwrap();
    assert_eq!(&buf, b"foo=ASDJKHQKBZXOQWEOPIUAXQWEOIU; max-age=3600; version=1");
}

