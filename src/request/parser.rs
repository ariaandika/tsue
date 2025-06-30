use std::io;

use crate::{method::Method, version::Version};


/// Parse result of [`parse_headline`].
#[derive(Debug)]
pub struct Headline<'a> {
    pub method: Method,
    pub uri: &'a str,
    pub version: Version,
}

pub fn parse_headline<'a>(buf: &mut &'a [u8]) -> io::Result<Option<Headline<'a>>> {
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
        // SAFETY: checked against static str
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

    Ok(Some(Headline {
        method,
        uri,
        version,
    }))
}

fn io_data_err<E: Into<Box<dyn std::error::Error + Send + Sync>>,>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_headline() {
        assert!(parse_headline(&mut &b"GE"[..]).unwrap().is_none());
        assert!(parse_headline(&mut &b"GET"[..]).unwrap().is_none());
        assert!(parse_headline(&mut &b"GET "[..]).unwrap().is_none());
        assert!(parse_headline(&mut &b"GET /users/g"[..]).unwrap().is_none());
        assert!(parse_headline(&mut &b"GET /users/get"[..]).unwrap().is_none());
        assert!(parse_headline(&mut &b"GET /users/get "[..]).unwrap().is_none());
        assert!(parse_headline(&mut &b"GET /users/get HTTP/1"[..]).unwrap().is_none());


        let mut buf = &b"GET /users/get HTTP/1.1\r\nHost: "[..];
        let ok = parse_headline(&mut buf).unwrap().unwrap();
        assert_eq!(ok.method, Method::GET);
        assert_eq!(ok.uri, "/users/get");
        assert_eq!(ok.version, Version::HTTP_11);
        assert_eq!(&buf, b"Host: ");
    }
}

