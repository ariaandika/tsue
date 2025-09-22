use std::task::Poll;
use tcio::bytes::BytesMut;

use super::Kind;
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
    use super::Reqline;

    macro_rules! test {
        (#[pending] $input:literal) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            match Reqline::matches(&mut bytes) {
                Poll::Pending => { }
                Poll::Ready(val) => panic!("expected `Poll::Pending`, but its ready with: {val:?}"),
            }
            assert_eq!(bytes.as_slice(), $input);
        };
        (#[error] $input:expr) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            match Reqline::matches(&mut bytes) {
                Poll::Ready(result) => result.unwrap_err(),
                Poll::Pending => panic!("line {}, unexpected Poll::Pending",line!()),
            }
        };
        {
            $input:expr;
            $m:ident, [$k:ident,$u:expr], $v:ident;
            $rest:expr
        } => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);

            let reqline = ready!(Reqline::matches(&mut bytes)).unwrap();

            assert_eq!(reqline.method, Method::$m);
            assert_eq!(reqline.target.kind, Kind::$k);
            assert_eq!(reqline.target.value.as_slice(), $u);
            assert_eq!(reqline.version, Version::$v);
            assert_eq!(bytes.as_slice(), $rest, "invalid remaining bytes");
        };
    }

    test! {
        b"GET / HTTP/1.1\r\n";
        GET, [Origin, b"/"], HTTP_11;
        b""
    };
    test! {
        b"GET / HTTP/1.1\n";
        GET, [Origin, b"/"], HTTP_11;
        b""
    };
    test! {
        b"GET / HTTP/1.1\r\nContent-Type: text/html\r\n";
        GET, [Origin, b"/"], HTTP_11;
        b"Content-Type: text/html\r\n"
    };
    test! {
        b"GET / HTTP/1.1\nContent-Type: text/html\r\n";
        GET, [Origin, b"/"], HTTP_11;
        b"Content-Type: text/html\r\n"
    };
    test! {
        b"GET /index.html HTTP/1.1\r\n";
        GET, [Origin, b"/index.html"], HTTP_11;
        b""
    };
    test! {
        b"GET /search?search=adequate&filter=available HTTP/1.1\r\n";
        GET, [Origin, b"/search?search=adequate&filter=available"], HTTP_11;
        b""
    };
    test! {
        b"GET /docs#section1 HTTP/1.1\r\nReferer: https://example.com\r\n";
        GET, [Origin, b"/docs#section1"], HTTP_11;
        b"Referer: https://example.com\r\n"
    };
    test! {
        b"OPTIONS * HTTP/2.0\r\nContent-Type: text/html\r\n";
        OPTIONS, [Asterisk, b"*"], HTTP_2;
        b"Content-Type: text/html\r\n"
    };
    test! {
        b"GET /old-page HTTP/1.0\r\nConnection: close\r\n";
        GET, [Origin, b"/old-page"], HTTP_10;
        b"Connection: close\r\n"
    };
    test! {
        b"GET /path%20with%20spaces HTTP/1.1\r\nContent-Type: text/plain\r\n";
        GET, [Origin, b"/path%20with%20spaces"], HTTP_11;
        b"Content-Type: text/plain\r\n"
    };
    test! {
        b"GET /user/john.doe@example.com HTTP/1.1\r\nAuth";
        GET, [Origin, b"/user/john.doe@example.com"], HTTP_11;
        b"Auth"
    };
    test! {
        b"GET /very/long/path/that/goes/on/for/many/characters/and/should/be/parsed/correctly HTTP/1.1\r\nX-Custom: value\r\n";
        GET, [Origin, b"/very/long/path/that/goes/on/for/many/characters/and/should/be/parsed/correctly"], HTTP_11;
        b"X-Custom: value\r\n"
    };

    // Error
    test!(#[error] b"GET / HTTP/1.1\rContent-Ty");
    test!(#[error] b"OPTIONS /users/all HTTP/1.1\rContent-Ty");

    test!(#[error] b"GET\n");
    test!(#[error] b"GET /\n");
    test!(#[error] b"GET HTTP/1.1\n");
    test!(#[error] b"GETHTTP/1.1\n");

    // Path is unchecked at this phase
    // test!(#[error] b"GET /users /all HTTP/1.1\n");
    // test!(#[error] b"GET /user\x7F/all HTTP/1.1\n");
    // test!(#[error] b"GET /user\x80/all HTTP/1.1\n");

    // Pending
    test!(#[pending] b"");
    test!(#[pending] b"GET / HTTP/1.1");
    test!(#[pending] b"GET / ");
    test!(#[pending] b"GET/\x00");
    test!(#[pending] b"GET/\r");
}

#[test]
fn test_parse_header() {
    use super::Header;

    macro_rules! test {
        (#[end] $input:literal, $remain:literal) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            assert!(ready!(Header::matches(&mut bytes)).unwrap().is_none());
            assert_eq!(bytes.as_slice(), $remain);
        };
        (#[pending] $input:literal) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            assert!(parse_reqline(&mut bytes).is_pending());
            assert_eq!(bytes.as_slice(), $input);
        };
        (#[error] $input:expr) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            match Header::matches(&mut bytes) {
                Poll::Ready(result) => assert!(result.is_err()),
                Poll::Pending => panic!("line {}, unexpected Poll::Pending",line!()),
            }
        };
        {
            $input:expr;
            $name:literal, $value:literal,
            $rest:expr
        } => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            let header = ready!(Header::matches(&mut bytes)).unwrap().unwrap();
            assert_eq!(&header.name, &$name[..]);
            assert_eq!(&header.value, &$value[..]);
            assert_eq!(bytes.as_slice(), $rest, "invalid remaining bytes");
        };
    }

    test! {
        b"Content-Length: 1224\r\nContent-Type: text/html\r\n\r\n";
        "Content-Length", b"1224",
        b"Content-Type: text/html\r\n\r\n"
    }

    test! {
        b"Content-Length: 1224\nContent-Type: text/html\n\r\n";
        "Content-Length", b"1224",
        b"Content-Type: text/html\n\r\n"
    }

    // test!(#[error] b"Content\x7FLength: 1224\nContent-Type: text/html\n\r\n");
    // test!(#[error] b"Content\x80Length: 1224\nContent-Type: text/html\n\r\n");

    test!(#[end] b"\r\nHello World!", b"Hello World!");
}

