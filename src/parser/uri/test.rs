use tcio::bytes::{ByteStr, Bytes};

#[test]
fn test_match_uri_leader() {
    use super::simd::match_uri_leader;

    macro_rules! test {
        {
            input: $i:expr,
            next: $n:expr,
            remain: $r:expr,
        } => {
            {
                let bytes = Bytes::from_static($i);
                let mut cursor = bytes.cursor();
                match_uri_leader(&mut cursor);
                assert_eq!(cursor.next(), $n);
                assert_eq!(cursor.as_slice(), $r);
            }
        };
    }

    test! {
        input: b"uri+scheme://example.com",
        next: Some(b':'),
        remain: b"//example.com",
    }
    test! {
        input: b"not scheme://example.com",
        next: Some(b' '),
        remain: b"scheme://example.com",
    }
    test! {
        input: b"scheme:\x7f//example.com",
        next: Some(b':'),
        remain: b"\x7f//example.com",
    }
    test! {
        input: b"scheme\x7f://example.com",
        next: Some(b'\x7f'),
        remain: b"://example.com",
    }
}

#[test]
fn test_parse_uri_origin() {
    use super::simd::match_path;

    macro_rules! test {
        {
            input: $i:expr,
            next: $n:expr,
            remain: $r:expr,
        } => {
            {
                let bytes = Bytes::from_static($i);
                let mut cursor = bytes.cursor();
                match_path(&mut cursor);
                assert_eq!(cursor.next(), $n);
                assert_eq!(cursor.as_slice(), $r);
            }
        };
    }

    test! {
        input: b"/users/all",
        next: None,
        remain: b"",
    }
    test! {
        input: b"/users/all?filter=available",
        next: Some(b'?'),
        remain: b"filter=available",
    }
    test! {
        input: b"/users/all#additional-section-4",
        next: Some(b'#'),
        remain: b"additional-section-4",
    }
    test! {
        input: b"/users/all?filter=available#additional-section-4",
        next: Some(b'?'),
        remain: b"filter=available#additional-section-4",
    }
    test! {
        input: b"/users/all#additional-section-4?filter=available",
        next: Some(b'#'),
        remain: b"additional-section-4?filter=available",
    }
    test! {
        input: b"/users/one for?filter=available",
        next: Some(b' '),
        remain: b"for?filter=available",
    }
    test! {
        input: b"/users/one\x1ffor?filter=available",
        next: Some(0x1f),
        remain: b"for?filter=available",
    }
}

#[test]
fn test_uri_parse() {
    use super::uri::{parse, Target};

    macro_rules! test_origin {
        (#[error] input: $i:expr) => {
            assert!(parse(ByteStr::copy_from_str($i)).is_err());
        };
        {
            input: $i:expr,
            path: $p:expr,
            query: $q:expr,
        } => {
            let Target::Origin(origin) = parse(ByteStr::copy_from_str($i)).unwrap() else {
                unreachable!("parse uri is not an origin form")
            };
            assert_eq!(origin.path(), $p);
            assert_eq!(origin.query(), $q);
        };
    }

    test_origin! {
        input: "/users/all",
        path: "/users/all",
        query: None,
    }
    test_origin! {
        input: "/",
        path: "/",
        query: None,
    }
    test_origin! {
        input: "/users/all?query=1&filter=available",
        path: "/users/all",
        query: Some("query=1&filter=available"),
    }
    test_origin! {
        input: "/users/all?",
        path: "/users/all",
        query: None,
    }
    test_origin! {
        input: "?query=1&filter=available",
        path: "/",
        query: Some("query=1&filter=available"),
    }
    test_origin! {
        input: "/users/all#additional-section-4",
        path: "/users/all",
        query: None,
    }
    test_origin! {
        input: "/users/all#",
        path: "/users/all",
        query: None,
    }
    test_origin! {
        input: "/users/all?query=1&filter=available#additional-section-4",
        path: "/users/all",
        query: Some("query=1&filter=available"),
    }

    test_origin! {
        #[error]
        input: ""
    }
}

