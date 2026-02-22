use tcio::bytes::{Buf, BytesMut};

use crate::http::Method;
use crate::proto::error::ParseError;

use ParseError as E;

mod matches;

// ===== Request Line =====

pub fn find_crlf(bytes: &mut BytesMut) -> Option<BytesMut> {
    // OPTIMIZE:use swar/simd for searching crlf
    let lf = bytes.iter().position(|&b| b == b'\n')?;
    if lf == 0 {
        bytes.advance(1);
        return Some(BytesMut::new());
    }
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
