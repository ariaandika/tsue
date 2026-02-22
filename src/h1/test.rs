use tcio::bytes::BytesMut;

#[test]
fn test_find_crlf() {
    use crate::h1::parser::find_crlf;

    macro_rules! test {
        ($data:expr, $expect:expr, $rest:expr) => {{
            let mut data = BytesMut::copy_from_slice($data);
            let line = find_crlf(&mut data).unwrap();
            assert_eq!(line.as_slice(), $expect);
            assert_eq!(data.as_slice(), $rest);
        }};
    }

    assert!(find_crlf(&mut BytesMut::copy_from_slice(b"GET / HTTP/1.")).is_none());

    // (input, result, rest)
    test!(b"\r\n", b"", b"");
    test!(b"\n", b"", b"");
    test!(b"GET / HTTP/1.1\r\n", b"GET / HTTP/1.1", b"");
    test!(b"GET / HTTP/1.1\n", b"GET / HTTP/1.1", b"");

    test!(
        b"GET / HTTP/1.1\r\nHost: example.com",
        b"GET / HTTP/1.1", b"Host: example.com"
    );
    test!(
        b"GET / HTTP/1.1\nHost: example.com",
        b"GET / HTTP/1.1", b"Host: example.com"
    );
}

#[test]
fn test_parse_reqline() {
    use crate::http::Method;
    use crate::h1::parser::parse_reqline;

    macro_rules! test {
        {
            $input:expr;
            $m:ident, [$k:ident,$u:expr]
        } => {
            let bytes = BytesMut::copy_from_slice($input);
            let (method, target) = parse_reqline(bytes).unwrap();
            assert_eq!(method, Method::$m);
            assert_eq!(target.as_slice(), $u);
        };
    }

    test! {
        b"GET / HTTP/1.1";
        GET, [Origin, b"/"]
    };
    test! {
        b"GET /index.html HTTP/1.1";
        GET, [Origin, b"/index.html"]
    };
    test! {
        b"GET /search?search=adequate&filter=available HTTP/1.1";
        GET, [Origin, b"/search?search=adequate&filter=available"]
    };
    test! {
        b"GET /docs#section1 HTTP/1.1";
        GET, [Origin, b"/docs#section1"]
    };
    test! {
        b"GET /path%20with%20spaces HTTP/1.1";
        GET, [Origin, b"/path%20with%20spaces"]
    };
    test! {
        b"GET /user/john.doe@example.com HTTP/1.1";
        GET, [Origin, b"/user/john.doe@example.com"]
    };
    test! {
        b"GET /very/long/path/that/goes/on/for/many/characters/and/should/be/parsed/correctly HTTP/1.1";
        GET, [Origin, b"/very/long/path/that/goes/on/for/many/characters/and/should/be/parsed/correctly"]
    };
}

#[test]
fn test_parse_header() {
    use super::parser::parse_header;

    macro_rules! test {
        (#[end] $input:literal, $remain:literal) => {
            let mut bytes = BytesMut::copy_from_slice(&$input[..]);
            assert!(ready!(parse_header_chunk(&mut bytes)).is_none());
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
            $name:expr, $value:expr,
        } => {
            let (name, val) = parse_header(BytesMut::copy_from_slice($input)).unwrap();
            assert_eq!(&name, $name);
            assert_eq!(&val, $value);
        };
    }

    test! {
        b"Content-Length: 1224";
        b"Content-Length", b"1224",
    }

    // test!(#[error] b"Content\x7FLength: 1224\nContent-Type: text/html\n\r\n");
    // test!(#[error] b"Content\x80Length: 1224\nContent-Type: text/html\n\r\n");

    // test!(#[end] b"\r\nHello World!", b"Hello World!");
}

