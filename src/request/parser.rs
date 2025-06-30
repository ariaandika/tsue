use std::io;

use crate::{method::Method, version::Version};


pub struct Headline<'a> {
    pub method: Method,
    pub uri: &'a str,
    pub version: Version,
    pub buf_len: usize,
}

pub fn parse_headline(buf: &[u8]) -> io::Result<Option<Headline>> {
    let mut bytes = buf;

    macro_rules! skip_space_after {
        (b"\r\n") => {
            if bytes.first_chunk::<2>() != Some(&b"\r\n") {
                return Err(io_data_err("expected cariage returns"))
            }
            bytes = &bytes[2..];
        };
        ($name:literal) => {
            if bytes.first() != Some(&b' ') {
                return Err(io_data_err(concat!("expected space after ", $name)))
            }
            bytes = &bytes[1..];
        };
    }

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

    skip_space_after!("method");

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

    skip_space_after!("uri");

    let version = {
        const VERSION_SIZE: usize = b"HTTP/".len();
        let Some((b"HTTP/", rest)) = bytes.split_first_chunk::<VERSION_SIZE>() else {
            return Ok(None);
        };
        let ok = match rest {
            [b'1',b'.',b'1',..] => Version::HTTP_11,
            [b'2',b'.',b'0',..] => Version::HTTP_2,
            [b'1',b'.',b'0',..] => Version::HTTP_10,
            [b'0',b'.',b'9',..] => Version::HTTP_09,
            _ => return Ok(None),
        };
        // SAFETY: checked against static value
        bytes = unsafe { bytes.get_unchecked(VERSION_SIZE + 3..) };
        ok
    };

    skip_space_after!(b"\r\n");

    Ok(Some(Headline {
        method,
        uri,
        version,
        buf_len: buf.len() - bytes.len(),
    }))
}

fn io_data_err<E: Into<Box<dyn std::error::Error + Send + Sync>>,>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_headline() {
        const BUF: &[u8] = b"GET /users/get HTTP/1.1\r\nHost: ";
        let ok = super::parse_headline(BUF).unwrap().unwrap();
        assert_eq!(ok.method, Method::GET);
        assert_eq!(ok.uri, "/users/get");
        assert_eq!(ok.version, Version::HTTP_11);
        assert_eq!(&BUF[ok.buf_len..], b"Host: ");
    }
}

