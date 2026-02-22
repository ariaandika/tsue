use tcio::bytes::{Buf, Bytes, BytesMut};

use crate::http::Method;
use crate::proto::error::ParseError;

use ParseError as E;

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
    let lf = find_crlf_swar(bytes)?;

    // SAFETY: `lf - 1` cannot overflow because `Some(b'\n') != bytes.first()`
    let cr = unsafe { *bytes.get_unchecked(lf - 1) == b'\r' } as usize;
    let mut reqline = bytes.split_to(lf + 1);
    unsafe { reqline.set_len(reqline.len() - 1 - cr) };
    Some(reqline)
}

pub fn parse_reqline(mut line: BytesMut) -> Result<(Method, Bytes), ParseError> {
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
    let Some(VER) = line.last_chunk() else {
        return Err(E::UnsupportedVersion);
    };
    line.truncate(line.len() - VER.len());

    Ok((method, line.freeze()))
}

pub fn parse_header(mut line: BytesMut) -> Result<(BytesMut, Bytes), ParseError> {
    let Some(col) = find_hdr_delim_swar(&line) else {
        return Err(E::InvalidSeparator);
    };
    let Some(b": ") = line.get(col..col + 2) else {
        return Err(E::InvalidSeparator);
    };
    let val = line.split_off(col + 2).freeze();
    line.truncate(col);
    Ok((line, val))
}

// ===== SWAR =====

const BLOCK: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
const LF: usize = usize::from_ne_bytes([b'\n'; BLOCK]);
const COL: usize = usize::from_ne_bytes([b':'; BLOCK]);

// OPTIMIZE:use simd for finding delimiter

fn find_crlf_swar(bytes: &[u8]) -> Option<usize> {
    let lf_ptr = 'swar: {
        let mut state = bytes;
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
    // helps BytesMut::split* bounds checking
    unsafe { std::hint::assert_unchecked(lf < bytes.len()) };
    Some(lf)
}

fn find_hdr_delim_swar(bytes: &[u8]) -> Option<usize> {
    let ptr = 'swar: {
        let mut state = bytes;
        while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
            let block = usize::from_ne_bytes(*chunk);
            // ':'
            let is_col = (block ^ COL).wrapping_sub(LSB) & MSB;
            if is_col != 0 {
                let nth = (is_col.trailing_zeros() / 8) as usize;
                break 'swar unsafe { chunk.as_ptr().add(nth) };
            }
            state = rest;
        }

        for byte in state {
            if let b':' = byte {
                break 'swar byte as *const u8;
            }
        }
        return None;
    };

    let idx = unsafe { ptr.offset_from_unsigned(bytes.as_ptr()) };
    // helps BytesMut::split* bounds checking
    unsafe { std::hint::assert_unchecked(idx < bytes.len()) };
    Some(idx)
}

