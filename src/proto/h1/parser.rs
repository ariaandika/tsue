//! HTTP Request Parser
use std::{io, mem::MaybeUninit};
use bytes::Bytes;
use tcio::{
    slice::{range_of, slice_of_bytes, slice_of_bytes_mut}, ByteStr
};

use crate::http::{Method, Version};

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(Some(ok)) => ok,
            Ok(None) => return Ok(None),
            Err(err) => return Err(err),
        }
    };
}

// ===== Request Line =====

/// Result of [`parse_line`].
#[derive(Debug)]
pub struct RequestLineRef<'a> {
    pub method: Method,
    pub uri: &'a str,
    pub version: Version,
}

/// Result of [`parse_line_buf`].
#[derive(Debug)]
pub struct RequestLine {
    pub method: Method,
    pub uri: tcio::ByteStr,
    pub version: Version,
}

pub fn parse_line_buf<B: bytes::Buf>(mut buf: B) -> io::Result<Option<RequestLine>> {
    let mut bytes = buf.chunk();

    let RequestLineRef {
        method,
        uri,
        version,
    } = tri!(parse_line(&mut bytes));

    let total_len = bytes.as_ptr() as usize - buf.chunk().as_ptr() as usize;
    let uri_offset = uri.as_ptr() as usize - buf.chunk().as_ptr() as usize;
    let uri_len = uri.len();
    let remaining = total_len - (uri_offset + uri_len);

    buf.advance(uri_offset);
    // SAFETY: `uri` is a `str`
    let uri = unsafe { ByteStr::from_utf8_unchecked(buf.copy_to_bytes(uri_len)) };

    buf.advance(remaining);

    Ok(Some(RequestLine {
        method,
        uri,
        version,
    }))
}

// TODO: simd request parser

/// Parse HTTP Request line.
pub fn parse_line<'a>(buf: &mut &'a [u8]) -> io::Result<Option<RequestLineRef<'a>>> {
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
            Ok(uri) => {
                bytes = &bytes[n..];
                uri
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

    {
        match bytes.first_chunk::<2>() {
            Some(crlf) => if crlf != b"\r\n" {
                return Err(io_data_err("expected cariage returns"));
            },
            None => return Ok(None),
        }
        // SAFETY: checked by `.first_chunk()`
        bytes = unsafe { bytes.get_unchecked(2..) };
    }

    *buf = bytes;

    Ok(Some(RequestLineRef {
        method,
        uri,
        version,
    }))
}

// ===== Header =====

/// Result of [`parse_header`].
#[derive(Debug, Clone)]
pub struct HeaderRef<'a> {
    pub name: &'a str,
    pub value: &'a [u8],
}

impl<'a> HeaderRef<'a> {
    /// Constant for empty [`HeaderRef`].
    pub const EMPTY: Self = Self { name: "", value: b"" };

    /// Returns a slice of `buf` containing the header name and value.
    pub fn slice_ref(&self, buf: &Bytes) -> HeaderBuf {
        HeaderBuf {
            name: ByteStr::from_slice_of(self.name, buf),
            value: buf.slice_ref(self.value),
        }
    }
}

/// Result of [`parse_header_buf`].
#[derive(Debug, Clone)]
pub struct HeaderBuf {
    pub name: ByteStr,
    pub value: Bytes,
}

impl HeaderBuf {
    /// Create new empty [`HeaderBuf`].
    pub const fn new() -> Self {
        Self { name: ByteStr::new(), value: Bytes::new() }
    }
}

impl Default for HeaderBuf {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_header_buf<B: bytes::Buf>(mut buf: B) -> io::Result<Option<HeaderBuf>> {
    let mut bytes = buf.chunk();
    let offset = bytes.as_ptr() as usize;

    let HeaderRef { name, value } = tri!(parse_header(&mut bytes));

    debug_assert_eq!(offset, name.as_ptr() as usize);

    let name_len = name.len();
    let value_offset = value.as_ptr() as usize - offset;
    let value_len = value.len();
    let sep_len = value_offset - name_len;

    // SAFETY: `name` is a `str`
    let name = unsafe { ByteStr::from_utf8_unchecked(buf.copy_to_bytes(name_len)) };
    buf.advance(sep_len);
    let value = buf.copy_to_bytes(value_len);

    debug_assert_eq!(&buf.chunk()[..2], b"\r\n");
    buf.advance(2);

    Ok(Some(HeaderBuf { name, value }))
}

/// Parse HTTP Header.
///
/// Note that this does not check for empty line which indicate the end of headers in HTTP.
pub fn parse_header<'a>(buf: &mut &'a [u8]) -> io::Result<Option<HeaderRef<'a>>> {
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
        let Some(cr) = bytes.iter().position(|e| e == &b'\r') else {
            return Ok(None);
        };
        match bytes.get(cr + 1) {
            Some(lf) => {
                if lf != &b'\n' {
                    return Err(io_data_err("unexpected cariage in header value"));
                }
            }
            None => return Ok(None),
        }
        // SAFETY: checked by `bytes.get(n + 1)`
        let ok = unsafe { bytes.get_unchecked(..cr) };
        // SAFETY: checked by `bytes.get(n + 1)`
        bytes = unsafe { bytes.get_unchecked(cr + 2..) };
        ok
    };

    *buf = bytes;

    Ok(Some(HeaderRef { name, value }))
}

#[inline]
pub fn parse_headers<'a, 'h>(
    buf: &mut &'a [u8],
    headers: &'h mut [HeaderRef<'a>],
) -> io::Result<Option<&'h mut [HeaderRef<'a>]>> {
    // SAFETY: `MaybeUninit<T>` is guaranteed to have the same size, alignment as `T`:
    let headers = unsafe { &mut *(headers as *mut [HeaderRef] as *mut [MaybeUninit<HeaderRef>]) };
    parse_headers_uninit(buf, headers)
}

pub fn parse_headers_uninit<'a, 'h>(
    buf: &mut &'a [u8],
    headers: &'h mut [MaybeUninit<HeaderRef<'a>>],
) -> io::Result<Option<&'h mut [HeaderRef<'a>]>> {
    let mut bytes = *buf;
    let mut n = 0;

    loop {
        if n >= headers.len() {
            break;
        }

        match bytes.first_chunk::<2>() {
            Some(b"\r\n") => {
                // SAFETY: checked by `first_chunk::<2>`
                bytes = unsafe { bytes.get_unchecked(2..) };
                break
            },
            Some(_) => {
                let Some(header) = parse_header(&mut bytes)? else {
                    return Ok(None);
                };
                headers[n].write(header);
                n += 1;
            }
            None => return Ok(None),
        }
    }

    *buf = bytes;
    // SAFETY: `n` is the amount of `MaybeUninit` that has been initialized
    let headers = unsafe { headers.get_unchecked_mut(..n) };
    // SAFETY: `MaybeUninit<T>` is guaranteed to have the same size, alignment as `T`:
    let headers = unsafe { &mut *(headers as *mut [MaybeUninit<HeaderRef>] as *mut [HeaderRef]) };
    Ok(Some(headers))
}

// ===== HeaderRange =====

/// Result of [`parse_header`].
#[derive(Debug)]
pub struct HeaderRange {
    // INVARIANT: range buffer is valid UTF-8
    name: std::ops::Range<usize>,
    value: std::ops::Range<usize>,
}

impl HeaderRange {
    fn new(header: &HeaderRef) -> Self {
        HeaderRange {
            // INVARIANT: range buffer is valid UTF-8
            name: range_of(header.name.as_bytes()),
            value: range_of(header.value),
        }
    }

    /// Returns the header name as [`ByteStr`] from given `bytes`.
    pub fn resolve_name(&self, bytes: &mut bytes::BytesMut) -> Result<ByteStr, std::str::Utf8Error> {
        tcio::ByteStr::from_utf8(slice_of_bytes_mut(self.name.clone(), bytes).freeze())
    }

    /// Returns the header name as [`ByteStr`] from given `bytes` without checking that the string
    /// contains valid UTF-8.
    ///
    /// # Safety
    ///
    /// The `bytes` should be the same with [`parse_headers_range`] and is not mutated.
    pub unsafe fn resolve_name_unchecked(&self, bytes: &Bytes) -> ByteStr {
        // SAFETY: user guarantees that the bytes is not modified,
        // thus invariant of `self.name` contains can be uphelp
        unsafe { tcio::ByteStr::from_utf8_unchecked(slice_of_bytes(self.name.clone(), bytes)) }
    }

    /// Returns the header value as [`ByteStr`] from given `bytes`.
    pub fn resolve_value(&self, bytes: &Bytes) -> Bytes {
        slice_of_bytes(self.value.clone(), bytes)
    }

    // /// Resolve the range with given `buf` to [`HeaderRef`].
    // pub fn resolve_ref<'b>(&self, buf: &'b [u8]) -> HeaderRef<'b> {
    //     HeaderRef {
    //         // SAFETY: invariant of `self.name` is a valid UTF-8
    //         name: unsafe { str::from_utf8_unchecked(slice_of(self.name.clone(), buf)) },
    //         value: slice_of(self.value.clone(), buf),
    //     }
    // }
}

#[inline]
pub fn parse_headers_range<'h>(
    buf: &mut &[u8],
    headers: &'h mut [HeaderRange],
) -> io::Result<Option<&'h mut [HeaderRange]>> {
    // SAFETY: `MaybeUninit<T>` is guaranteed to have the same size, alignment as `T`:
    let headers = unsafe { &mut *(headers as *mut [HeaderRange] as *mut [MaybeUninit<HeaderRange>]) };
    parse_headers_range_uninit(buf, headers)
}

pub fn parse_headers_range_uninit<'h>(
    buf: &mut &[u8],
    headers: &'h mut [MaybeUninit<HeaderRange>],
) -> io::Result<Option<&'h mut [HeaderRange]>> {
    let mut bytes = *buf;
    let mut n = 0;

    loop {
        if n >= headers.len() {
            break;
        }

        match bytes.first_chunk::<2>() {
            Some(b"\r\n") => {
                // SAFETY: checked by `first_chunk::<2>`
                bytes = unsafe { bytes.get_unchecked(2..) };
                break;
            }
            Some(_) => {
                let Some(header) = parse_header(&mut bytes)? else {
                    return Ok(None);
                };
                headers[n].write(HeaderRange::new(&header));
                n += 1;
            }
            None => return Ok(None),
        }
    }

    *buf = bytes;
    let headers = &mut headers[..n];
    // SAFETY: `MaybeUninit<T>` is guaranteed to have the same size, alignment as `T`:
    let headers = unsafe { &mut *(headers as *mut [MaybeUninit<HeaderRange>]as *mut [HeaderRange]) };
    Ok(Some(headers))
}

fn io_data_err<E: Into<Box<dyn std::error::Error + Send + Sync>>,>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e)
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use super::*;

    #[test]
    fn test_parse_line() {
        assert!(parse_line_buf(&b""[..]).unwrap().is_none());
        assert!(parse_line_buf(&b"GE"[..]).unwrap().is_none());
        assert!(parse_line_buf(&b"GET"[..]).unwrap().is_none());
        assert!(parse_line_buf(&b"GET "[..]).unwrap().is_none());
        assert!(parse_line_buf(&b"GET /users/g"[..]).unwrap().is_none());
        assert!(parse_line_buf(&b"GET /users/get"[..]).unwrap().is_none());
        assert!(parse_line_buf(&b"GET /users/get "[..]).unwrap().is_none());
        assert!(parse_line_buf(&b"GET /users/get HTTP/1"[..]).unwrap().is_none());
        assert!(parse_line_buf(&b"GET /users/get HTTP/1.1"[..]).unwrap().is_none());
        assert!(parse_line_buf(&b"GET /users/get HTTP/1.1\r"[..]).unwrap().is_none());


        let mut buf = &b"GET /users/get HTTP/1.1\r\nHost: "[..];
        let ok = parse_line(&mut buf).unwrap().unwrap();
        assert_eq!(ok.method, Method::GET);
        assert_eq!(ok.uri, "/users/get");
        assert_eq!(ok.version, Version::HTTP_11);
        assert_eq!(&buf, b"Host: ");


        let mut buf = BytesMut::from(&b"GET /users/get HTTP/1.1\r\nHost: "[..]);
        let ok = parse_line_buf(&mut buf).unwrap().unwrap();
        assert_eq!(ok.method, Method::GET);
        assert_eq!(ok.uri, "/users/get");
        assert_eq!(ok.version, Version::HTTP_11);
        assert_eq!(&buf[..], b"Host: ");
    }

    #[test]
    fn test_parse_line_method() {
        assert_eq!(parse_line_buf(&b"GET / HTTP/1.1\r\n"[..]).unwrap().unwrap().method, Method::GET);
        assert_eq!(parse_line_buf(&b"PUT / HTTP/1.1\r\n"[..]).unwrap().unwrap().method, Method::PUT);
        assert_eq!(parse_line_buf(&b"POST / HTTP/1.1\r\n"[..]).unwrap().unwrap().method, Method::POST);
        assert_eq!(parse_line_buf(&b"HEAD / HTTP/1.1\r\n"[..]).unwrap().unwrap().method, Method::HEAD);
        assert_eq!(parse_line_buf(&b"PATCH / HTTP/1.1\r\n"[..]).unwrap().unwrap().method, Method::PATCH);
        assert_eq!(parse_line_buf(&b"TRACE / HTTP/1.1\r\n"[..]).unwrap().unwrap().method, Method::TRACE);
        assert_eq!(parse_line_buf(&b"DELETE / HTTP/1.1\r\n"[..]).unwrap().unwrap().method, Method::DELETE);
        assert_eq!(parse_line_buf(&b"CONNECT / HTTP/1.1\r\n"[..]).unwrap().unwrap().method, Method::CONNECT);
        assert_eq!(parse_line_buf(&b"OPTIONS / HTTP/1.1\r\n"[..]).unwrap().unwrap().method, Method::OPTIONS);
    }

    #[test]
    fn test_parse_header() {
        assert!(parse_header_buf(&b""[..]).unwrap().is_none());
        assert!(parse_header_buf(&b"Hos"[..]).unwrap().is_none());
        assert!(parse_header_buf(&b"Host"[..]).unwrap().is_none());
        assert!(parse_header_buf(&b"Host:"[..]).unwrap().is_none());
        assert!(parse_header_buf(&b"Host: "[..]).unwrap().is_none());
        assert!(parse_header_buf(&b"Host: loca"[..]).unwrap().is_none());
        assert!(parse_header_buf(&b"Host: localhost"[..]).unwrap().is_none());
        assert!(parse_header_buf(&b"Host: localhost\r"[..]).unwrap().is_none());


        let mut buf = &b"Host: localhost\r\nConte"[..];
        let ok = parse_header(&mut buf).unwrap().unwrap();
        assert_eq!(ok.name, "Host");
        assert_eq!(ok.value, b"localhost");
        assert_eq!(&buf, b"Conte");


        let mut buf = BytesMut::from(&b"Host: localhost\r\nConte"[..]);
        let ok = parse_header_buf(&mut buf).unwrap().unwrap();
        assert_eq!(ok.name, "Host");
        assert_eq!(&ok.value[..], b"localhost");
        assert_eq!(&buf[..], b"Conte");
    }

    const HEADERS: &str = "\
        Host: localhost\r\n\
        Content-Type: text/html\r\n\
        \r\n\
        Hello World!\
    ";

    #[test]
    fn test_parse_headers() {
        let mut buf = &HEADERS.as_bytes()[..16];
        let mut headers = [HeaderRef::EMPTY;4];

        assert!(parse_headers(&mut buf, &mut headers).unwrap().is_none());

        let buf = Bytes::copy_from_slice(HEADERS.as_bytes());
        let mut chunk = &buf[..];

        let sliced = parse_headers(&mut chunk, &mut headers).unwrap().unwrap();

        assert_eq!(sliced[0].name, "Host");
        assert_eq!(sliced[0].value, b"localhost");
        assert_eq!(sliced[1].name, "Content-Type");
        assert_eq!(sliced[1].value, b"text/html");
        assert_eq!(chunk, b"Hello World!");

        let mut headers_buf = [const { HeaderBuf::new() };4];

        for (i, header_ref) in sliced.iter().enumerate() {
            headers_buf[i] = header_ref.slice_ref(&buf);
        }

        // no copy
        assert_eq!(sliced[0].name.as_ptr(), headers_buf[0].name.as_ptr());
        assert_eq!(sliced[0].value.as_ptr(), headers_buf[0].value.as_ptr());

        assert_eq!(headers_buf[0].name, "Host");
        assert_eq!(&headers_buf[0].value[..], b"localhost");
        assert_eq!(headers_buf[1].name, "Content-Type");
        assert_eq!(&headers_buf[1].value[..], b"text/html");
        assert_eq!(chunk, b"Hello World!");
    }

    #[test]
    fn test_parse_headers_range() {
        let mut buf = HEADERS.as_bytes();
        let mut headers = [const { MaybeUninit::uninit() };4];

        let _sliced = parse_headers_range_uninit(&mut buf, &mut headers).unwrap().unwrap();

        // let host = sliced[0].resolve_ref(HEADERS.as_bytes());
        // assert_eq!(host.name, "Host");
        // assert_eq!(host.value, b"localhost");
    }
}

