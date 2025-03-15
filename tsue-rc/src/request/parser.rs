use super::Parts;
use crate::bytestr::ByteStr;
use crate::http::{Header, Method, Version, HEADER_SIZE};
use bytes::{Buf, BytesMut};
use std::str::Utf8Error;

/// parse http request
///
/// return `Ok(None)` when buffer end before parse complete
pub fn parse(buf: &mut BytesMut) -> Result<Option<Parts>,ParseError> {
    macro_rules! try_advance {
        ($n:literal) => {
            match buf.len() >= $n {
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
                if match buf.get(i) {
                    Some(some) => some.$($tt)*,
                    None => return Ok(None),
                } {
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
        _ => return Err(format!("invalid method: {method:?}").into())
    };

    log::trace!("parsed method {method:?}");

    // wh
    try_advance!(1);

    let path = collect_word!();
    let path = ByteStr::from_bytes(path.freeze())?;

    log::trace!("parsed path {path:?}");

    // wh
    try_advance!(1);

    let version = collect_word!();
    let version = match &version[..] {
        b"HTTP/1.0" => Version::Http10,
        b"HTTP/1.1" => Version::Http11,
        b"HTTP/2" => Version::Http2,
        _ => return Err(format!("invalid version: {version:?}").into()),
    };

    try_advance!(2);

    log::trace!("parsed version {version:?}");

    // headers
    let mut headers = [const { Header::new_static() };HEADER_SIZE];
    let mut header_len = 0;
    loop {
        if header_len >= HEADER_SIZE { break; }

        if matches!((buf.first(),buf.get(1)),(Some(b'\r'),Some(&b'\n'))) {
            buf.advance(2);
            break;
        }

        let mut name = collect_word!(eq(&b':'));
        name.make_ascii_lowercase();
        let name = ByteStr::from_bytes(name.freeze())?;

        try_advance!(2);

        let value = collect_word!().freeze();

        headers[header_len] = Header { name, value };
        header_len += 1;

        try_advance!(2);
        log::trace!("parsed header {:?}",&headers[header_len-1]);
    }

    Ok(Some(Parts {
        method,
        path,
        version,
        headers,
        header_len,
    }))
}

/// error maybe return from [`parse`]
#[derive(Debug)]
pub struct ParseError(String);

impl From<String> for ParseError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<Utf8Error> for ParseError {
    fn from(value: Utf8Error) -> Self {
        Self(value.to_string())
    }
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::fmt(&self.0, f)
    }
}

