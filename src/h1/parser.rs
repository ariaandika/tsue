use tcio::bytes::{Buf, BytesMut};

use crate::http::Method;
use crate::proto::error::ParseError;

use ParseError as E;

mod matches;

const BLOCK: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
const LF: usize = usize::from_ne_bytes([b'\n'; BLOCK]);

// OPTIMIZE:use simd for h1 parsing

const MIN_REQLINE_LEN: usize = b"GET / HTTP/1.1".len();

pub fn find_crlf(bytes: &mut BytesMut) -> Option<BytesMut> {
    // header list termination
    if let Some(b'\n') = bytes.first() {
        bytes.advance(1);
        return Some(BytesMut::new());
    }
    if let Some(b"\r\n") = bytes.first_chunk() {
        bytes.advance(2);
        return Some(BytesMut::new());
    }
    if bytes.len() < MIN_REQLINE_LEN {
        return None;
    }

    let lf_ptr = 'swar: {
        let mut state = bytes.as_slice();

        while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
            let block = usize::from_ne_bytes(*chunk);

            // '\n'
            let is_lf = (block ^ LF).wrapping_sub(LSB) & MSB;

            if is_lf != 0 {
                let nth = (is_lf.trailing_zeros() / 8) as usize;
                break 'swar unsafe { chunk.as_ptr().add(nth) };
            }

            state = rest;
        }

        for byte in state {
            if let b'\n' = byte {
                break 'swar byte as *const u8;
            }
        }

        return None;
    };

    let lf = unsafe { lf_ptr.offset_from_unsigned(bytes.as_ptr()) };
    let cr = (bytes.get(lf - 1) == Some(&b'\r')) as usize;
    let reqline = bytes.split_to(lf - cr);
    bytes.advance(1 + cr);
    Some(reqline)
}

pub fn parse_reqline(mut line: BytesMut) -> Result<(Method, BytesMut), ParseError> {
    let method = 'method: {
        if line.first_chunk() == Some(b"GET ") {
            line.advance(4);
            break 'method Method::GET;
        }
        if line.first_chunk() == Some(b"POST ") {
            line.advance(5);
            break 'method Method::POST;
        }

        let len = line.iter().position(|&e|e == b' ').ok_or(E::InvalidSeparator)?;
        let method = Method::from_bytes(&line[..len]).ok_or(E::UnknownMethod)?;
        line.advance(len + 1);
        method
    };

    const VER: &[u8; 9] = b" HTTP/1.1";
    if line.last_chunk() != Some(VER) {
        return Err(E::UnsupportedVersion);
    }
    line.truncate(line.len() - VER.len());

    Ok((method, line))
}

pub fn parse_header(mut line: BytesMut) -> Result<(BytesMut, BytesMut), ParseError> {
    // OPTIMIZE:use swar/simd for searching space
    let sp = line.iter().position(|&b| b == b' ').ok_or(E::InvalidSeparator)?;
    if sp == 0 {
        return Err(E::InvalidHeader);
    }
    if line.get(sp - 1..sp + 1) != Some(b": ") {
        return Err(E::InvalidSeparator);
    }
    let val = line.split_off(sp + 2);
    line.truncate(sp - 1);
    Ok((line, val))
}
