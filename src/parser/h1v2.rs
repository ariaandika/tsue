use std::slice;
use tcio::bytes::{BytesMut, Cursor};

use crate::http::{Method, Version};

const CHUNK_SIZE: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([128; CHUNK_SIZE]);
const LSB: usize = usize::from_ne_bytes([1; CHUNK_SIZE]);

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(Some(ok)) => ok,
            Ok(None) => return Ok(None),
            Err(err) => return Err(err),
        }
    };
}

macro_rules! err {
    ($variant:ident) => {
        Err(Error { kind: ErrorKind::$variant })
    };
}

#[allow(unused, reason = "later")]
pub fn parse_request(buf: &mut BytesMut) -> Result<Option<()>, Error> {
    let mut bytes = buf.as_slice();

    let reqline = t!(parse_reqline(&mut bytes));

    todo!()
}

type Reqline<'a> = (Method, &'a [u8], Version);

fn parse_reqline<'a>(bytes: &mut &'a [u8]) -> Result<Option<Reqline<'a>>, Error> {
    let Some(reqline) = find_reqline(bytes) else {
        return Ok(None);
    };

    let (method, method_len) = {
        let Some((lead, rest)) = reqline.split_first_chunk() else {
            return err!(TooShort);
        };

        let result = match lead {
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
                _ => return err!(UnknownMethod),
            },
        };

        // even if this oob in `reqline`, it will point to b'\n' in original slice
        if unsafe { *reqline.as_ptr().add(result.1) } != b' ' {
            return err!(InvalidSeparator);
        }

        result
    };

    const VERSION_SIZE: usize = b"HTTP/1.1".len();
    const VERSION_SIZE_SP: usize = b" HTTP/1.1".len();

    let version = {
        let Some(version) = reqline.last_chunk::<VERSION_SIZE>() else {
            return err!(TooShort);
        };

        match version {
            b"HTTP/1.1" => Version::HTTP_11,
            b"HTTP/2.0" => Version::HTTP_2,
            b"HTTP/3.0" => Version::HTTP_3,
            b"HTTP/1.0" => Version::HTTP_10,
            b"HTTP/0.9" => Version::HTTP_09,
            _ => return err!(UnsupportedVersion),
        }
    };

    let Some(uri_len) = reqline
        .len()
        .checked_sub(VERSION_SIZE_SP + method_len + 1/* 1st sp */)
    else {
        return err!(InvalidSeparator);
    };
    if uri_len == 0 {
        return err!(InvalidSeparator);
    }

    let uri = unsafe { slice::from_raw_parts(reqline.as_ptr().add(method_len + 1), uri_len) };

    Ok(Some((method, uri, version)))
}

const fn find_reqline<'a>(bytes: &mut &'a [u8]) -> Option<&'a [u8]> {
    const LF: usize = usize::from_ne_bytes([b'\n'; CHUNK_SIZE]);

    let mut cursor = Cursor::new(bytes);

    while let Some(chunk) = cursor.peek_chunk::<CHUNK_SIZE>() {
        let value = usize::from_ne_bytes(*chunk);
        let lf_xor = value ^ LF;
        let lf_result = lf_xor.wrapping_sub(LSB) & !lf_xor & MSB;

        if lf_result != 0 {
            let pos = (lf_result.trailing_zeros() / 8) as usize;

            cursor.advance(pos + 1);
            *bytes = cursor.as_slice();

            if let Some(b"\r\n") = cursor.peek_prev_chunk() {
                cursor.step_back(2);
            } else {
                cursor.step_back(1);
            }

            return Some(cursor.advanced_slice());
        }

        cursor.advance(CHUNK_SIZE);
    }

    while let Some(b) = cursor.next() {
        if b == b'\n' {
            *bytes = cursor.as_slice();

            if let Some(b"\r\n") = cursor.peek_prev_chunk() {
                cursor.step_back(2);
            } else {
                cursor.step_back(1);
            }

            return Some(cursor.advanced_slice());
        }
    }

    None
}

// ===== Error =====

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.kind {
            ErrorKind::UnknownMethod => f.write_str("unknown method"),
            ErrorKind::TooShort => f.write_str("request line too short"),
            ErrorKind::UnsupportedVersion => f.write_str("unsupported HTTP version"),
            ErrorKind::InvalidSeparator => f.write_str("invalid separator"),
        }
    }
}

#[derive(Debug)]
enum ErrorKind {
    UnknownMethod,
    /// Request line is too short.
    TooShort,
    /// HTTP Version unsupported.
    UnsupportedVersion,
    /// Request line have invalid separator
    InvalidSeparator
}

// ===== Test =====

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! test_parse_reqline {
        {
            $input:expr
        } => {
            let mut bytes = &$input[..];
            parse_reqline(&mut bytes).unwrap_err();
        };
        {
            $input:expr;
            $m:ident, $u:expr, $v:ident;
            $rest:expr
        } => {
            let mut bytes = &$input[..];

            let reqline = parse_reqline(&mut bytes).unwrap().unwrap();

            assert_eq!(reqline.0, Method::$m);
            assert_eq!(reqline.1, $u);
            assert_eq!(reqline.2, Version::$v);
            assert_eq!(bytes, $rest);
        };
    }

    #[test]
    fn test_reqline() {
        test_parse_reqline! {
            b"GET / HTTP/1.1\r\nContent-Type: text/html\r\n";
            GET, b"/", HTTP_11;
            b"Content-Type: text/html\r\n"
        };
        test_parse_reqline! {
            b"GET / HTTP/1.1\nContent-Type: text/html\r\n";
            GET, b"/", HTTP_11;
            b"Content-Type: text/html\r\n"
        };
        test_parse_reqline! {
            b"GET / HTTP/1.1\r\n";
            GET, b"/", HTTP_11;
            b""
        };
        test_parse_reqline! {
            b"GET / HTTP/1.1\n";
            GET, b"/", HTTP_11;
            b""
        };
        test_parse_reqline! {
            b"OPTIONS /user/all HTTP/2.0\r\nContent-Type: text/html\r\n";
            OPTIONS, b"/user/all", HTTP_2;
            b"Content-Type: text/html\r\n"
        };
        // Error
        test_parse_reqline!(b"GET /HTTP/1.1\n");
        test_parse_reqline!(b"GET\n");
        test_parse_reqline!(b"HTTP/1.1\n");
        test_parse_reqline!(b"GETHTTP/1.1\n");
        test_parse_reqline!(b"GET HTTP/1.1\n");
    }
}

