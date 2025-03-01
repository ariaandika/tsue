use super::{Header, Method, ReqParts, Version, MAX_HEADER};
use crate::bytestring::ByteStr;
use bytes::{Buf, Bytes, BytesMut};
use std::str::Utf8Error;

macro_rules! partial {
    ($e:expr) => {
        match $e {
            Some(some) => some,
            None => return Ok(None),
        }
    };
}

/// parse request
///
/// return `Ok(None)` when buffer end before parse complete
pub fn parse_request(buf: &mut BytesMut) -> Result<Option<ReqParts>,ParseError> {
    use ParseError::*;

    macro_rules! try_advance {
        ($n:literal) => {
            match buf.len() == $n {
                true => buf.advance($n),
                false => return Ok(None),
            }
        };
    }

    macro_rules! collect_word {
        () => {
            collect_word!(is_ascii_whitespace())
        };
        ($($tt:tt)*) => {{
            let mut i = 0;
            loop {
                if partial!(buf.get(i)).$($tt)* {
                    break buf.split_to(i);
                }
                i += 1;
            }
        }};
    }

    // NOTE: method

    let method = collect_word!();
    let method = match &method[..] {
        b"GET" | b"get" => Method::GET,
        b"POST" | b"post" => Method::POST,
        b"PUT" | b"put" => Method::PUT,
        b"PATCH" | b"patch" => Method::PUT,
        b"DELETE" | b"delete" => Method::DELETE,
        b"HEAD" | b"head" => Method::HEAD,
        b"CONNECT" | b"connect" => Method::CONNECT,
        _ => return Err(InvalidMethod(method.freeze()))
    };

    // wh
    try_advance!(1);

    let path = collect_word!();
    let path = ByteStr::from_bytes(path.freeze())?;

    // wh
    try_advance!(1);

    let version = collect_word!();
    let version = match &version[..] {
        b"HTTP/1.0" => Version::Http10,
        b"HTTP/1.1" => Version::Http11,
        b"HTTP/2" => Version::Http2,
        _ => return Err(InvalidVersion(version.freeze())),
    };

    try_advance!(2);

    // headers
    let mut headers = [const { Header::new() };MAX_HEADER];
    let mut header_len = 0;
    loop {
        if header_len >= MAX_HEADER { break; }

        let mut name = collect_word!(eq(&b':'));
        name.make_ascii_lowercase();
        let name = ByteStr::from_bytes(name.freeze())?;

        try_advance!(2);
        let value = collect_word!(eq(&b':')).freeze();

        headers[header_len] = Header { name, value };
        header_len += 1;
    }

    Ok(Some(ReqParts {
        method,
        path,
        version,
        headers,
        header_len,
    }))
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("invalid method: {0:?}")]
    InvalidMethod(Bytes),
    #[error("invalid path: {0}")]
    InvalidPath(#[from] Utf8Error),
    #[error("invalid version: {0:?}")]
    InvalidVersion(Bytes),
}

