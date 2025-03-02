//! the [`request::Parts`] and [`Request`] type
//!
//! [`request::Parts`]: Parts
use super::{request, Header, Method, Version, MAX_HEADER};
use crate::{body::Body, bytestring::ByteStr};
use bytes::{Buf, Bytes, BytesMut};
use std::str::Utf8Error;

#[derive(Default)]
pub struct Parts {
    method: Method,
    path: ByteStr,
    version: Version,
    headers: [Header;MAX_HEADER],
    header_len: usize,
}

impl Parts {
    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn headers(&self) -> &[Header] {
        &self.headers[..self.header_len]
    }

    pub fn path(&self) -> &ByteStr {
        &self.path
    }

    pub fn version(&self) -> &Version {
        &self.version
    }
}

#[derive(Default)]
pub struct Request {
    parts: Parts,
    body: Body,
}

impl Request {
    pub fn from_parts(parts: Parts, body: Body) -> Request {
        Self { parts, body  }
    }

    pub fn into_parts(self) -> (Parts,Body) {
        (self.parts,self.body)
    }

    pub fn into_body(self) -> Body {
        self.body
    }
}


/// parse request
///
/// return `Ok(None)` when buffer end before parse complete
pub fn parse(buf: &mut BytesMut) -> Result<Option<request::Parts>,ParseError> {
    use ParseError::*;

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
        _ => return Err(InvalidMethod(method.freeze()))
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
        _ => return Err(InvalidVersion(version.freeze())),
    };

    try_advance!(2);

    log::trace!("parsed version {version:?}");

    // headers
    let mut headers = [const { Header::new_static() };MAX_HEADER];
    let mut header_len = 0;
    loop {
        if header_len >= MAX_HEADER { break; }

        if matches!((buf.get(0),buf.get(1)),(Some(b'\r'),Some(&b'\n'))) {
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

    Ok(Some(request::Parts {
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

impl std::fmt::Debug for Parts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parts")
            .field("method", &self.method)
            .field("path", &self.path)
            .field("version", &self.version)
            .field("headers", &self.headers())
            .finish()
    }
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.parts.method)
            .field("path", &self.parts.path)
            .field("version", &self.parts.version)
            .field("headers", &self.parts.headers())
            .finish()
    }
}

