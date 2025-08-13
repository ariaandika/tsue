use std::task::Poll;
use tcio::bytes::BytesMut;

use super::{message::find_line_buf, request::parse_reqline};
use crate::{http::{Method, Version}, parser::h1::header::parse_header};

#[test]
fn test_find_line() {
    let mut bytes = BytesMut::copy_from_slice(b"Content-Length: 1224\r\nContent-Type: text/html\r\n");
    let line = find_line_buf(&mut bytes).unwrap();
    assert_eq!(line.as_slice(), b"Content-Length: 1224");
    assert_eq!(bytes.as_slice(), b"Content-Type: text/html\r\n");

    let mut bytes = BytesMut::copy_from_slice(b"Content-Length: 1224\nContent-Type: text/html\n");
    let line = find_line_buf(&mut bytes).unwrap();
    assert_eq!(line.as_slice(), b"Content-Length: 1224");
    assert_eq!(bytes.as_slice(), b"Content-Type: text/html\n");
}

macro_rules! test_parse_reqline {
    (@error $input:expr) => {
        let mut bytes = BytesMut::copy_from_slice(&$input[..]);
        let Poll::Ready(result) = parse_reqline(&mut bytes) else { unreachable!() };
        result.unwrap_err();
    };
    {
        $input:expr;
        $m:ident, $u:expr, $v:ident;
        $rest:expr
    } => {
        let mut bytes = BytesMut::copy_from_slice(&$input[..]);

        let Poll::Ready(result) = parse_reqline(&mut bytes) else { unreachable!() };
        let reqline = result.unwrap();

        assert_eq!(reqline.method, Method::$m);
        assert_eq!(reqline.target.as_slice(), $u);
        assert_eq!(reqline.version, Version::$v);
        assert_eq!(bytes.as_slice(), $rest);
    };
}

#[test]
fn test_parse_reqline() {
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
    test_parse_reqline!(@error b"GET /HTTP/1.1\n");
    test_parse_reqline!(@error b"GET\n");
    test_parse_reqline!(@error b"HTTP/1.1\n");
    test_parse_reqline!(@error b"GETHTTP/1.1\n");
    test_parse_reqline!(@error b"GET HTTP/1.1\n");
}

#[test]
fn test_parse_header() {
    const HEADERS: &[u8] = b"Content-Length: 1224\r\nContent-Type: text/html\r\n\r\n";

    let mut bytes = BytesMut::copy_from_slice(b"Content-Length\r");
    assert!(parse_header(&mut bytes).is_pending());

    let mut bytes = BytesMut::copy_from_slice(HEADERS);

    let header = parse_header(&mut bytes).unwrap().unwrap().unwrap();
    assert_eq!(header.name.as_slice(), b"Content-Length");
    assert_eq!(header.value.as_slice(), b"1224");

    let header = parse_header(&mut bytes).unwrap().unwrap().unwrap();
    assert_eq!(header.name.as_slice(), b"Content-Type");
    assert_eq!(header.value.as_slice(), b"text/html");

    assert!(parse_header(&mut bytes).unwrap().is_none());
}

trait PollExt<T> {
    fn unwrap(self) -> T;
}

impl<T> PollExt<T> for Poll<T> {
    fn unwrap(self) -> T {
        match self {
            Poll::Ready(ok) => ok,
            Poll::Pending => unreachable!()
        }
    }
}

