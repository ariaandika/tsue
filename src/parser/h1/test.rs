use std::task::Poll;
use tcio::bytes::BytesMut;

use crate::http::{Method, Version};

macro_rules! ready {
    ($e:expr) => {
        match $e {
            Poll::Ready(ok) => ok,
            Poll::Pending => panic!("called `Poll::unwrap` on `Poll::Pending`")
        }
    };
}

#[test]
fn test_parse_reqline() {
    use super::request::parse_reqline;

    macro_rules! test {
        (#[pending] $input:literal) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            assert!(parse_reqline(&mut bytes).is_pending());
            assert_eq!(bytes.as_slice(), $input);
        };
        (#[error] $input:expr) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            match parse_reqline(&mut bytes) {
                Poll::Ready(result) => result.unwrap_err(),
                Poll::Pending => panic!("line {}, unexpected Poll::Pending",line!()),
            }
        };
        {
            $input:expr;
            $m:ident, $u:expr, $v:ident;
            $rest:expr
        } => {
            todo!()
            // let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            //
            // let reqline = ready!(parse_reqline(&mut bytes)).unwrap();
            //
            // assert_eq!(reqline.method, Method::$m);
            // let Target::Origin(target) = reqline.target else { unreachable!() };
            // assert_eq!(target.path_and_query().as_bytes(), $u);
            // assert_eq!(reqline.version, Version::$v);
            // assert_eq!(bytes.as_slice(), $rest, "invalid remaining bytes");
        };
    }

    test! {
        b"GET / HTTP/1.1\r\n";
        GET, b"/", HTTP_11;
        b""
    };
    test! {
        b"GET / HTTP/1.1\n";
        GET, b"/", HTTP_11;
        b""
    };
    test! {
        b"GET / HTTP/1.1\r\nContent-Type: text/html\r\n";
        GET, b"/", HTTP_11;
        b"Content-Type: text/html\r\n"
    };
    test! {
        b"GET / HTTP/1.1\nContent-Type: text/html\r\n";
        GET, b"/", HTTP_11;
        b"Content-Type: text/html\r\n"
    };
    test! {
        b"GET /index.html HTTP/1.1\r\n";
        GET, b"/index.html", HTTP_11;
        b""
    };
    test! {
        b"GET /search?search=adequate&filter=available HTTP/1.1\r\n";
        GET, b"/search?search=adequate&filter=available", HTTP_11;
        b""
    };
    test! {
        b"OPTIONS * HTTP/2.0\r\nContent-Type: text/html\r\n";
        OPTIONS, b"*", HTTP_2;
        b"Content-Type: text/html\r\n"
    };
    // Error
    test!(#[error] b"GET / HTTP/1.1\rContent-Ty");
    test!(#[error] b"OPTIONS /users/all HTTP/1.1\rContent-Ty");
    test!(#[error] b" / HTTP/1.1\r\nContent-Ty");
    test!(#[error] b"GET /HTTP/1.1\n");
    test!(#[error] b"GET\n");
    test!(#[error] b"HTTP/1.1\n");
    test!(#[error] b"GETHTTP/1.1\n");
    test!(#[error] b"GET HTTP/1.1\n");
    // Pending
    test!(#[pending] b"");
    test!(#[pending] b"GET/\x00");
    test!(#[pending] b"GET/\r");
    test!(#[pending] b"GET / HTTP/1.1");
    test!(#[pending] b"GET / ");
}

#[test]
fn test_parse_header() {
    use super::header::parse_header;

    macro_rules! test {
        (#[end] $input:literal, $remain:literal) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            assert!(ready!(parse_header(&mut bytes)).unwrap().is_none());
            assert_eq!(bytes.as_slice(), $remain);
        };
        (#[pending] $input:literal) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            assert!(parse_reqline(&mut bytes).is_pending());
            assert_eq!(bytes.as_slice(), $input);
        };
        (#[error] $input:expr) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            match parse_reqline(&mut bytes) {
                Poll::Ready(result) => result.unwrap_err(),
                Poll::Pending => panic!("line {}, unexpected Poll::Pending",line!()),
            }
        };
        {
            $input:expr;
            $name:literal, $value:literal,
            $rest:expr
        } => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);

            let header = ready!(parse_header(&mut bytes)).unwrap().unwrap();

            assert_eq!(&header.name, &$name[..]);
            assert_eq!(&header.value, &$value[..]);
            assert_eq!(bytes.as_slice(), $rest, "invalid remaining bytes");
        };
    }

    test! {
        b"Content-Length: 1224\r\nContent-Type: text/html\r\n\r\n";
        b"Content-Length", b"1224",
        b"Content-Type: text/html\r\n\r\n"
    }

    test!(#[end] b"\r\nHello World!", b"Hello World!");

    // const HEADERS: &[u8] = b"Content-Length: 1224\r\nContent-Type: text/html\r\n\r\n";
    //
    // let mut bytes = BytesMut::copy_from_slice(b"Content-Length\r");
    // assert!(parse_header(&mut bytes).is_pending());
    //
    // let mut bytes = BytesMut::copy_from_slice(HEADERS);
    //
    // let header = ready!(parse_header(&mut bytes)).unwrap().unwrap();
    // assert_eq!(header.name.as_slice(), b"Content-Length");
    // assert_eq!(header.value.as_slice(), b"1224");
    //
    // let header = ready!(parse_header(&mut bytes)).unwrap().unwrap();
    // assert_eq!(header.name.as_slice(), b"Content-Type");
    // assert_eq!(header.value.as_slice(), b"text/html");
    //
    // assert!(ready!(parse_header(&mut bytes)).unwrap().is_none());
}

