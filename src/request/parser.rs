//! HTTP Request Parser
use std::io;

use crate::{method::Method, version::Version};

/// Parse result of [`parse_line`].
#[derive(Debug)]
pub struct RequestLine<'a> {
    pub method: Method,
    pub uri: &'a str,
    pub version: Version,
}

/// Parse HTTP Request line.
#[inline]
pub fn parse_line_ref(mut buf: &[u8]) -> io::Result<Option<RequestLine<'_>>> {
    parse_line(&mut buf)
}

/// Parse HTTP Request line.
pub fn parse_line<'a>(buf: &mut &'a [u8]) -> io::Result<Option<RequestLine<'a>>> {
    let mut bytes = *buf;

    let method = {
        let Some((lead, rest)) = bytes.split_first_chunk::<4>() else {
            return Ok(None);
        };
        let (ok, len) = match lead {
            b"GET " => (Method::GET, 3),
            b"PUT " => (Method::PUT, 3),
            b"POST" => (Method::POST, 4),
            b"HEAD" => (Method::HEAD, 4),
            _ => match (lead, rest) {
                (b"PATC", [b'H', ..]) => (Method::PATCH, 5),
                (b"TRAC", [b'E', ..]) => (Method::TRACE, 5),
                (b"DELE", [b'T', b'E', ..]) => (Method::DELETE, 6),
                (b"CONN", [b'E', b'C', b'T', ..]) => (Method::CONNECT, 7),
                (b"OPTI", [b'O', b'N', b'S', ..]) => (Method::OPTIONS, 7),
                _ => return Ok(None),
            },
        };
        // SAFETY: `len` is acquired from `lead`, `lead` is guaranteed in `bytes` by
        // `.split_first_chunk::<4>()`
        bytes = unsafe { bytes.get_unchecked(len..) };
        ok
    };

    if bytes.first() != Some(&b' ') {
        return Err(io_data_err("expected space after method"));
    } else {
        // SAFETY: checked by `.first()`
        bytes = unsafe { bytes.get_unchecked(1..) };
    }

    let uri = {
        let Some(n) = bytes.iter().position(|e| e == &b' ') else {
            return Ok(None)
        };
        match str::from_utf8(&bytes[..n]) {
            Ok(ok) => {
                bytes = &bytes[n..];
                ok
            },
            Err(e) => return Err(io_data_err(e)),
        }
    };

    if bytes.first() != Some(&b' ') {
        return Err(io_data_err("expected space after uri"));
    } else {
        // SAFETY: checked by `.first()`
        bytes = unsafe { bytes.get_unchecked(1..) };
    }

    let version = {
        const VERSION_SIZE: usize = b"HTTP/".len();
        let Some((b"HTTP/", rest)) = bytes.split_first_chunk::<VERSION_SIZE>() else {
            return Ok(None);
        };
        let ok = match rest {
            [b'1', b'.', b'1', ..] => Version::HTTP_11,
            [b'2', b'.', b'0', ..] => Version::HTTP_2,
            [b'1', b'.', b'0', ..] => Version::HTTP_10,
            [b'0', b'.', b'9', ..] => Version::HTTP_09,
            _ => return Ok(None),
        };
        // SAFETY: checked against static value
        bytes = unsafe { bytes.get_unchecked(VERSION_SIZE + 3..) };
        ok
    };

    if bytes.first_chunk::<2>() != Some(b"\r\n") {
        return Err(io_data_err("expected cariage returns"));
    } else {
        // SAFETY: checked by `.first_chunk()`
        bytes = unsafe { bytes.get_unchecked(2..) };
    }

    *buf = bytes;

    Ok(Some(RequestLine {
        method,
        uri,
        version,
    }))
}

/// Parse result of [`parse_header`].
#[derive(Debug)]
pub struct Header<'a> {
    pub name: &'a str,
    pub value: &'a [u8],
}

/// Parse HTTP Header.
///
/// Note that this does not check for empty line which indicate the end of headers in HTTP.
pub fn parse_header<'a>(buf: &mut &'a [u8]) -> io::Result<Option<Header<'a>>> {
    let mut bytes = *buf;

    let name = {
        let Some(n) = bytes.iter().position(|e| e == &b':') else {
            return Ok(None);
        };
        match str::from_utf8(&bytes[..n]) {
            Ok(ok) => {
                bytes = &bytes[n..];
                ok
            }
            Err(e) => return Err(io_data_err(e)),
        }
    };

    {
        let Some(sepr) = bytes.first_chunk::<2>() else {
            return Ok(None)
        };
        if sepr != b": " {
            return Err(io_data_err("expected space after colon"));
        } else {
            // SAFETY: checked by `.first_chunk::<2>()`
            bytes = unsafe { bytes.get_unchecked(2..) };
        }
    }

    let value = {
        let Some(n) = bytes.iter().position(|e| e == &b'\r') else {
            return Ok(None);
        };
        match bytes.get(n + 1) {
            Some(lf) => {
                if lf != &b'\n' {
                    return Err(io_data_err("unexpected cariage in header value"));
                }
            }
            None => return Ok(None),
        }
        let ok = unsafe { bytes.get_unchecked(..n) };
        bytes = unsafe { bytes.get_unchecked(n + 2..) };
        ok
    };

    *buf = bytes;

    Ok(Some(Header { name, value }))
}

fn io_data_err<E: Into<Box<dyn std::error::Error + Send + Sync>>,>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_line() {
        assert!(parse_line_ref(b"").unwrap().is_none());
        assert!(parse_line_ref(b"GE").unwrap().is_none());
        assert!(parse_line_ref(b"GET").unwrap().is_none());
        assert!(parse_line_ref(b"GET ").unwrap().is_none());
        assert!(parse_line_ref(b"GET /users/g").unwrap().is_none());
        assert!(parse_line_ref(b"GET /users/get").unwrap().is_none());
        assert!(parse_line_ref(b"GET /users/get ").unwrap().is_none());
        assert!(parse_line_ref(b"GET /users/get HTTP/1").unwrap().is_none());


        let mut buf = &b"GET /users/get HTTP/1.1\r\nHost: "[..];
        let ok = parse_line(&mut buf).unwrap().unwrap();
        assert_eq!(ok.method, Method::GET);
        assert_eq!(ok.uri, "/users/get");
        assert_eq!(ok.version, Version::HTTP_11);
        assert_eq!(&buf, b"Host: ");
    }

    #[test]
    fn test_parse_header() {
        assert!(parse_header(&mut &b""[..]).unwrap().is_none());
        assert!(parse_header(&mut &b"Hos"[..]).unwrap().is_none());
        assert!(parse_header(&mut &b"Host"[..]).unwrap().is_none());
        assert!(parse_header(&mut &b"Host:"[..]).unwrap().is_none());
        assert!(parse_header(&mut &b"Host: "[..]).unwrap().is_none());
        assert!(parse_header(&mut &b"Host: loca"[..]).unwrap().is_none());
        assert!(parse_header(&mut &b"Host: localhost"[..]).unwrap().is_none());


        let mut buf = &b"Host: localhost\r\nConte"[..];
        let ok = parse_header(&mut buf).unwrap().unwrap();
        assert_eq!(ok.name, "Host");
        assert_eq!(ok.value, b"localhost");
        assert_eq!(&buf, b"Conte");
    }
}

