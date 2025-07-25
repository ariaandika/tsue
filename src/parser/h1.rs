//! HTTP Request Parser
//!
//! # Parsing
//!
//! This module provide parsing http [request line][rl] and headers.
//!
//! The main parsing functions have signature of `fn parse(buf: &mut &[u8]) -> Result<Option<T>>`.
//!
//! The argument `buf: &mut &[u8]` can represent how many data is consumed while parsing. If the
//! parsing is incomplete, `buf` is unchanged.
//!
//! The return type is `Result<Option<T>>` which can represent an error, incomplete parsing
//! (`Ok(None)`), and success. When parsing is incomplete, more data read is required then parsing
//! can be retried.
//!
//! The main parsing functions are:
//!
//! - [`parse_line`] -> [`RequestLine`]
//! - [`parse_header`] -> [`HeaderRef`]
//!
//! # Slice Indexing
//!
//! Returning shared reference to the buffer as parsing result will prevent the buffer from being
//! mutated. This is a problem for achieving no-copy parsing strategy.
//!
//! We can workaround this by storing the index of the resulting slice. As long as the buffer
//! content is not mutated or reallocated, the index should points to the correct data, while
//! removing lifetime bounds.
//!
//! The function [`range_of`], [`slice_of_bytes`], and [`slice_of_bytes_mut`] can help with this.
//!
//! There is also [`parse_headers_range`] which achieve the same goal.
//!
//! # Other APIs
//!
//! Other APIs for integration with other types:
//!
//! - [`parse_line_buf`], work with [`bytes::Buf`]
//! - [`parse_header_buf`], work with [`bytes::Buf`]
//! - [`parse_headers`], parse multiple headers
//! - [`parse_headers_uninit`], integration with [`MaybeUninit`]
//! - [`parse_headers_range`], returning slice range
//! - [`parse_headers_range_uninit`], returning slice range with [`MaybeUninit`]
//!
//! # Examples
//!
//! ```rust
//! # use bytes::{BytesMut, Buf};
//! # use std::mem::MaybeUninit;
//! # use tsue::{
//! #       request::Parts,
//! #       http::{Uri, Extensions},
//! #       headers::{HeaderMap, HeaderName, HeaderValue}};
//! # use tcio::ByteStr;
//! use tsue::proto::h1::parser::{
//!     self, RequestLine,
//!     range_of, slice_of_bytes, slice_of_bytes_mut
//! };
//!
//! fn parse_request(buffer: &mut BytesMut) {
//!     let mut chunk = buffer.chunk();
//!
//!     let Some(RequestLine {
//!         method,
//!         uri,
//!         version,
//!     }) = parser::parse_line(&mut chunk).unwrap() else {
//!         todo!("read more")
//!     };
//!
//!     let uri_range = range_of(uri.as_bytes());
//!
//!     let mut headers = [const { MaybeUninit::uninit() }; 64];
//!     let Some(headers_range) = parser::parse_headers_range_uninit(&mut chunk, &mut headers)
//!         .unwrap()
//!     else {
//!         todo!("read more")
//!     };
//!
//!     // parsing complete, no more retry
//!
//!     let read = chunk.as_ptr() as usize - buffer.chunk().as_ptr() as usize;
//!     let mut buffer = buffer.split_to(read);
//!
//!     // Uri
//!     let uri_bytes = slice_of_bytes_mut(uri_range, &mut buffer).freeze();
//!     let uri = Uri::try_from_shared(ByteStr::from_utf8(uri_bytes).unwrap()).unwrap();
//!
//!     // Headers
//!     let mut headers = HeaderMap::new();
//!
//!     for header in headers_range {
//!         let name = header.resolve_name(&mut buffer).unwrap();
//!         let value = header.resolve_value_mut(&mut buffer).freeze();
//!
//!         if let Ok(value) = HeaderValue::try_from_slice(value) {
//!             headers.insert(name, value);
//!         }
//!     }
//!
//!     // Parse Complete
//!     let parts = Parts {
//!         method,
//!         uri,
//!         version,
//!         headers,
//!         extensions: Extensions::new(),
//!     };
//! }
//! ```
//!
//! [rl]: <https://httpwg.org/specs/rfc9112.html#request.line>
//! [`BytesMut`]: bytes::BytesMut
//! [`MaybeUninit`]: std::mem::MaybeUninit
use bytes::Bytes;
use std::{io, mem::MaybeUninit};
use tcio::{ByteStr, bytes::Cursor};

use crate::http::{Method, Version};

const _MIN_REQ_LEN: usize = "GET / HTTP/1.1\n".len();

pub use tcio::bytes::{range_of, slice_of_bytes, slice_of_bytes_mut};

macro_rules! t {
    ($e:expr) => {
        match $e {
            Some(ok) => ok,
            None => return Ok(None),
        }
    };
}

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
pub struct RequestLine<'a> {
    pub method: Method,
    pub uri: &'a str,
    pub version: Version,
}

/// Parse HTTP Request line.
///
/// [httpwg](https://httpwg.org/specs/rfc9112.html#request.line)
pub fn parse_line<'a>(buf: &mut &'a [u8]) -> Result<Option<RequestLine<'a>>, RequestLineError> {
    use RequestLineError::*;
    let mut cursor = Cursor::new(buf);

    // The method token is case-sensitive
    // Though this library only support standardized methods

    let method = {
        let lead = t!(cursor.next_chunk::<4>());
        let (ok, peek) = match lead {
            b"GET " => (Method::GET, 0),
            b"PUT " => (Method::PUT, 0),
            b"POST" => (Method::POST, 0),
            b"HEAD" => (Method::HEAD, 0),
            _ => match (lead, cursor.as_bytes()) {
                (b"PATC", [b'H', ..]) => (Method::PATCH, 1),
                (b"TRAC", [b'E', ..]) => (Method::TRACE, 1),
                (b"DELE", [b'T', b'E', ..]) => (Method::DELETE, 2),
                (b"CONN", [b'E', b'C', b'T', ..]) => (Method::CONNECT, 3),
                (b"OPTI", [b'O', b'N', b'S', ..]) => (Method::OPTIONS, 3),
                _ => return Err(UnknownMethod)
            },
        };

        // SAFETY: `len` is valid constant value
        unsafe { cursor.advance(peek) };

        ok
    };

    // Although the request-line grammar rule requires that each of the component elements be
    // separated by a single SP octet, recipients MAY instead parse on whitespace-delimited word
    // boundaries and, aside from the CRLF terminator, treat any form of whitespace as the SP
    // separator while ignoring preceding or trailing whitespace; such whitespace includes one or
    // more of the following octets: SP, HTAB, VT (%x0B), FF (%x0C), or bare CR.

    // Note that were gonna ignore it, we only accept single space separator

    match cursor.next() {
        Some(b' ') => {},
        Some(w) if w.is_ascii_whitespace() => return Err(InvalidSeparator),
        Some(_) => return Err(UnknownMethod),
        None => return Ok(None),
    }

    let uri = {
        let uri = t!(cursor.next_split(b' '));

        match str::from_utf8(uri) {
            Ok(uri) => uri,
            Err(_) => return Err(NonUtf8),
        }
    };

    let version = {
        const VERSION_SIZE: usize = b"HTTP/".len();

        let b"HTTP/" = t!(cursor.next_chunk::<VERSION_SIZE>()) else {
            return Err(InvalidToken)
        };

        match t!(cursor.next_chunk::<3>()) {
            [b'1', b'.', b'1'] => Version::HTTP_11,
            [b'2', b'.', b'0'] => Version::HTTP_2,
            [b'3', b'.', b'0'] => Version::HTTP_2,
            [b'1', b'.', b'0'] => Version::HTTP_10,
            [b'0', b'.', b'9'] => Version::HTTP_09,
            _ => return Err(InvalidToken),
        }
    };

    match t!(cursor.peek_chunk::<2>()) {
        b"\r\n" => {
            // SAFETY: checked by `.first_chunk::<2>()`
            unsafe { cursor.advance(2) };
        }
        [b'\n', field] if field.is_ascii_whitespace() => {
            return Err(InvalidSeparator);
        }
        [b'\n', _] => {
            // SAFETY: checked by `.first_chunk::<2>()`
            unsafe { cursor.advance(1) };
        }
        _ => return Err(InvalidSeparator),
    }

    *buf = cursor.as_bytes();

    Ok(Some(RequestLine {
        method,
        uri,
        version,
    }))
}

#[derive(Debug)]
pub enum RequestLineError {
    /// A server that receives a method longer than any that it implements SHOULD respond with a
    /// 501 (Not Implemented) status code.
    UnknownMethod,
    /// A server that receives a request-target longer than any URI it wishes to parse MUST respond
    /// with a 414 (URI Too Long) status code
    UriTooLong,
    /// This library only recognize single space separator, other than that is an error.
    InvalidSeparator,
    /// Request target or header name is not valid UTF-8.
    NonUtf8,
    /// Contains invalid token.
    InvalidToken,
}

impl std::error::Error for RequestLineError { }

impl std::fmt::Display for RequestLineError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::UnknownMethod => f.write_str("unknown method"),
            Self::UriTooLong => f.write_str("uri too long"),
            Self::InvalidSeparator => f.write_str("invalid separator"),
            Self::NonUtf8 => f.write_str("non utf8 bytes"),
            Self::InvalidToken => f.write_str("invalid token"),
        }
    }
}

// ===== Header =====

/// Result of [`parse_header`].
#[derive(Debug, Clone)]
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

    Ok(Some(Header { name, value }))
}

// ===== Utilities =====

/// Result of [`parse_line_buf`].
#[derive(Debug)]
pub struct RequestLineBuf {
    pub method: Method,
    pub uri: tcio::ByteStr,
    pub version: Version,
}

/// Parse HTTP Request line with [`bytes::Buf`].
pub fn parse_line_buf<B: bytes::Buf>(mut buf: B) -> Result<Option<RequestLineBuf>, RequestLineError> {
    let mut bytes = buf.chunk();

    let RequestLine {
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

    Ok(Some(RequestLineBuf {
        method,
        uri,
        version,
    }))
}

impl<'a> Header<'a> {
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

/// Parse header with [`bytes::Buf`].
pub fn parse_header_buf<B: bytes::Buf>(mut buf: B) -> io::Result<Option<HeaderBuf>> {
    let mut bytes = buf.chunk();
    let offset = bytes.as_ptr() as usize;

    let Header { name, value } = tri!(parse_header(&mut bytes));

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

/// Parse multiple headers.
#[inline]
pub fn parse_headers<'a, 'h>(
    buf: &mut &'a [u8],
    headers: &'h mut [Header<'a>],
) -> io::Result<Option<&'h mut [Header<'a>]>> {
    // SAFETY: `MaybeUninit<T>` is guaranteed to have the same size, alignment as `T`:
    let headers = unsafe { &mut *(headers as *mut [Header] as *mut [MaybeUninit<Header>]) };
    parse_headers_uninit(buf, headers)
}

/// Parse multiple headers with uninit slice.
pub fn parse_headers_uninit<'a, 'h>(
    buf: &mut &'a [u8],
    headers: &'h mut [MaybeUninit<Header<'a>>],
) -> io::Result<Option<&'h mut [Header<'a>]>> {
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
    let headers = unsafe { &mut *(headers as *mut [MaybeUninit<Header>] as *mut [Header]) };
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
    fn new(header: &Header) -> Self {
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

    /// Returns the header value as [`ByteStr`] from given `bytes`.
    pub fn resolve_value_mut(&self, bytes: &mut bytes::BytesMut) -> bytes::BytesMut {
        slice_of_bytes_mut(self.value.clone(), bytes)
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

/// Parse multiple headers returning a slice ranges.
#[inline]
pub fn parse_headers_range<'h>(
    buf: &mut &[u8],
    headers: &'h mut [HeaderRange],
) -> io::Result<Option<&'h mut [HeaderRange]>> {
    // SAFETY: `MaybeUninit<T>` is guaranteed to have the same size, alignment as `T`:
    let headers = unsafe { &mut *(headers as *mut [HeaderRange] as *mut [MaybeUninit<HeaderRange>]) };
    parse_headers_range_uninit(buf, headers)
}

/// Parse multiple headers returning a slice ranges with uninit slice.
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

        assert!(parse_line_buf(&b"OPTIONS"[..]).unwrap().is_none());


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
        let mut headers = [Header::EMPTY;4];

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

