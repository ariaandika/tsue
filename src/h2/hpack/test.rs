use tcio::bytes::{Bytes, BytesMut};

use crate::h2::hpack::Decoder;
use crate::headers::{HeaderMap, HeaderName, standard};

macro_rules! field_eq {
    ($field:expr, $name:literal, $val:literal) => {{
        let field = $field;
        assert_eq!(field.name().as_str(), $name);
        assert_eq!(field.value().as_str(), $val);
    }};
    ($field:expr, $std:ident, $val:literal) => {{
        let field = $field;
        assert_eq!(field.name().as_str(), standard::$std.as_str());
        assert_eq!(field.value().as_str(), $val);
    }};
}

/// https://httpwg.org/specs/rfc7541.html#n-literal-header-field-with-indexing
#[test]
fn test_decode_literal_header_field_with_indexing() {
    let mut table = Decoder::default();
    let mut write_buffer = BytesMut::new();
    let mut bytes = Bytes::copy_from_slice(&[
        0x40, 0x0a, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d, 0x2d, 0x6b, 0x65, 0x79, 0x0d, 0x63, 0x75,
        0x73, 0x74, 0x6f, 0x6d, 0x2d, 0x68, 0x65, 0x61, 0x64, 0x65, 0x72,
    ]);

    let field = table.decode_test(&mut bytes, &mut write_buffer).unwrap();
    field_eq!(field, "custom-key", "custom-header");

    assert_eq!(table.size(), 55);
    let ([entry], []) = table.fields().as_slices() else {
        unreachable!()
    };
    field_eq!(entry, "custom-key", "custom-header");
}

/// https://httpwg.org/specs/rfc7541.html#n-literal-header-field-without-indexing
#[test]
fn test_decode_literal_header_field_without_indexing() {
    let mut table = Decoder::default();
    let mut write_buffer = BytesMut::new();
    let mut bytes = Bytes::copy_from_slice(&[
        0x04, 0x0c, 0x2f, 0x73, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x70, 0x61, 0x74, 0x68,
    ]);

    let field = table.decode_test(&mut bytes, &mut write_buffer).unwrap();
    field_eq!(field, PSEUDO_PATH, "/sample/path");

    assert!(table.fields().is_empty());
    assert_eq!(table.size(), 0);
}

/// https://httpwg.org/specs/rfc7541.html#n-literal-header-field-never-indexed
#[test]
fn test_decode_literal_header_field_never_indexed() {
    let mut table = Decoder::default();
    let mut write_buffer = BytesMut::new();
    let mut bytes = Bytes::copy_from_slice(&[
        0x10, 0x08, 0x70, 0x61, 0x73, 0x73, 0x77, 0x6f, 0x72, 0x64, 0x06, 0x73, 0x65, 0x63, 0x72,
        0x65, 0x74,
    ]);

    let field = table.decode_test(&mut bytes, &mut write_buffer).unwrap();
    field_eq!(field, "password", "secret");

    assert!(table.fields().is_empty());
    assert_eq!(table.size(), 0);
}

/// https://httpwg.org/specs/rfc7541.html#n-literal-header-field-never-indexed
#[test]
fn test_decode_indexed_header_field() {
    let mut table = Decoder::default();
    let mut write_buffer = BytesMut::new();
    let mut bytes = Bytes::copy_from_slice(&[0x82]);

    let field = table.decode_test(&mut bytes, &mut write_buffer).unwrap();
    field_eq!(field, PSEUDO_METHOD, "GET");

    assert!(table.fields().is_empty());
    assert_eq!(table.size(), 0);
}

#[test]
fn test_decode_request_without_huffman() {
    const REQ1: [u8; 20] = [
        0x82, 0x86, 0x84, 0x41, 0x0f, 0x77, 0x77, 0x77, 0x2e, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c,
        0x65, 0x2e, 0x63, 0x6f, 0x6d,
    ];
    const REQ2: [u8; 14] = [
        0x82, 0x86, 0x84, 0xbe, 0x58, 0x08, 0x6e, 0x6f, 0x2d, 0x63, 0x61, 0x63, 0x68, 0x65,
    ];
    const REQ3: [u8; 29] = [
        0x82, 0x87, 0x85, 0xbf, 0x40, 0x0a, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d, 0x2d, 0x6b, 0x65,
        0x79, 0x0c, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d, 0x2d, 0x76, 0x61, 0x6c, 0x75, 0x65,
    ];
    test_decode_request(&REQ1, &REQ2, &REQ3);
}

#[test]
fn test_decode_request_with_huffman() {
    const REQ1: [u8; 17] = [
        0x82, 0x86, 0x84, 0x41, 0x8c, 0xf1, 0xe3, 0xc2, 0xe5, 0xf2, 0x3a, 0x6b, 0xa0, 0xab, 0x90,
        0xf4, 0xff,
    ];
    const REQ2: [u8; 12] = [
        0x82, 0x86, 0x84, 0xbe, 0x58, 0x86, 0xa8, 0xeb, 0x10, 0x64, 0x9c, 0xbf,
    ];
    const REQ3: [u8; 24] = [
        0x82, 0x87, 0x85, 0xbf, 0x40, 0x88, 0x25, 0xa8, 0x49, 0xe9, 0x5b, 0xa9, 0x7d, 0x7f, 0x89,
        0x25, 0xa8, 0x49, 0xe9, 0x5b, 0xb8, 0xe8, 0xb4, 0xbf,
    ];
    test_decode_request(&REQ1, &REQ2, &REQ3);
}

fn test_decode_request(req1: &[u8], req2: &[u8], req3: &[u8]) {
    let mut table = Decoder::default();
    let mut write_buffer = BytesMut::new();

    // first request
    {
        let mut map = HeaderMap::new();
        let mut bytes = Bytes::copy_from_slice(req1);

        while !bytes.is_empty() {
            let field = table.decode(&mut bytes, &mut write_buffer).unwrap();
            map.try_append_field(field.into_owned()).unwrap();
        }

        let mut fields = table.fields().iter();
        field_eq!(fields.next().unwrap(), PSEUDO_AUTHORITY, "www.example.com");
        assert!(fields.next().is_none());

        assert_eq!(table.size(), 57);
        assert_eq!(map.get(standard::PSEUDO_METHOD).unwrap(), &b"GET"[..]);
        assert_eq!(map.get(standard::PSEUDO_SCHEME).unwrap(), &b"http"[..]);
        assert_eq!(map.get(standard::PSEUDO_PATH).unwrap(), &b"/"[..]);
        assert_eq!(
            map.get(standard::PSEUDO_AUTHORITY).unwrap(),
            &b"www.example.com"[..]
        );
    }

    // second request
    {
        let mut map = HeaderMap::new();
        let mut bytes = Bytes::copy_from_slice(req2);

        while !bytes.is_empty() {
            let field = table.decode(&mut bytes, &mut write_buffer).unwrap();
            map.try_append_field(field.into_owned()).unwrap();
        }

        let mut fields = table.fields().iter();
        field_eq!(fields.next().unwrap(), CACHE_CONTROL, "no-cache");
        field_eq!(fields.next().unwrap(), PSEUDO_AUTHORITY, "www.example.com");
        assert!(fields.next().is_none());

        assert_eq!(table.size(), 110);
        assert_eq!(map.get(standard::PSEUDO_METHOD).unwrap(), &b"GET"[..]);
        assert_eq!(map.get(standard::PSEUDO_SCHEME).unwrap(), &b"http"[..]);
        assert_eq!(map.get(standard::PSEUDO_PATH).unwrap(), &b"/"[..]);
        assert_eq!(
            map.get(standard::PSEUDO_AUTHORITY).unwrap(),
            &b"www.example.com"[..]
        );
        assert_eq!(map.get(standard::CACHE_CONTROL).unwrap(), &b"no-cache"[..]);
    }

    // third request
    {
        let mut map = HeaderMap::new();
        let mut bytes = Bytes::copy_from_slice(req3);

        while !bytes.is_empty() {
            let field = table.decode(&mut bytes, &mut write_buffer).unwrap();
            map.try_append_field(field.into_owned()).unwrap();
        }

        let mut fields = table.fields().iter();
        field_eq!(fields.next().unwrap(), "custom-key", "custom-value");
        field_eq!(fields.next().unwrap(), CACHE_CONTROL, "no-cache");
        field_eq!(fields.next().unwrap(), PSEUDO_AUTHORITY, "www.example.com");
        assert!(fields.next().is_none());

        assert_eq!(table.size(), 164);
        assert_eq!(map.get(standard::PSEUDO_METHOD).unwrap(), &b"GET"[..]);
        assert_eq!(map.get(standard::PSEUDO_SCHEME).unwrap(), &b"https"[..]);
        assert_eq!(map.get(standard::PSEUDO_PATH).unwrap(), &b"/index.html"[..]);
        assert_eq!(
            map.get(standard::PSEUDO_AUTHORITY).unwrap(),
            &b"www.example.com"[..]
        );
        assert_eq!(
            map.get(HeaderName::from_static(b"custom-key")).unwrap(),
            &b"custom-value"[..]
        );
    }
}

#[test]
fn test_decode_response_without_huffman() {
    const RES1: [u8; 70] = [
        0x48, 0x03, 0x33, 0x30, 0x32, 0x58, 0x07, 0x70, 0x72, 0x69, 0x76, 0x61, 0x74, 0x65, 0x61,
        0x1d, 0x4d, 0x6f, 0x6e, 0x2c, 0x20, 0x32, 0x31, 0x20, 0x4f, 0x63, 0x74, 0x20, 0x32, 0x30,
        0x31, 0x33, 0x20, 0x32, 0x30, 0x3a, 0x31, 0x33, 0x3a, 0x32, 0x31, 0x20, 0x47, 0x4d, 0x54,
        0x6e, 0x17, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x77, 0x77, 0x77, 0x2e, 0x65,
        0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d,
    ];
    const RES2: [u8; 8] = [0x48, 0x03, 0x33, 0x30, 0x37, 0xc1, 0xc0, 0xbf];
    const RES3: [u8; 98] = [
        0x88, 0xc1, 0x61, 0x1d, 0x4d, 0x6f, 0x6e, 0x2c, 0x20, 0x32, 0x31, 0x20, 0x4f, 0x63, 0x74,
        0x20, 0x32, 0x30, 0x31, 0x33, 0x20, 0x32, 0x30, 0x3a, 0x31, 0x33, 0x3a, 0x32, 0x32, 0x20,
        0x47, 0x4d, 0x54, 0xc0, 0x5a, 0x04, 0x67, 0x7a, 0x69, 0x70, 0x77, 0x38, 0x66, 0x6f, 0x6f,
        0x3d, 0x41, 0x53, 0x44, 0x4a, 0x4b, 0x48, 0x51, 0x4b, 0x42, 0x5a, 0x58, 0x4f, 0x51, 0x57,
        0x45, 0x4f, 0x50, 0x49, 0x55, 0x41, 0x58, 0x51, 0x57, 0x45, 0x4f, 0x49, 0x55, 0x3b, 0x20,
        0x6d, 0x61, 0x78, 0x2d, 0x61, 0x67, 0x65, 0x3d, 0x33, 0x36, 0x30, 0x30, 0x3b, 0x20, 0x76,
        0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x3d, 0x31,
    ];
    test_decode_response(&RES1, &RES2, &RES3);
}

fn test_decode_response(res1: &[u8], res2: &[u8], res3: &[u8]) {
    let mut table = Decoder::new(256);
    let mut write_buffer = BytesMut::new();

    // first response
    {
        let mut map = HeaderMap::new();
        let mut bytes = Bytes::copy_from_slice(res1);

        while !bytes.is_empty() {
            let field = table.decode(&mut bytes, &mut write_buffer).unwrap();
            map.try_append_field(field.into_owned()).unwrap();
        }

        let mut table_iter = table.fields().iter();
        field_eq!(
            table_iter.next().unwrap(),
            LOCATION,
            "https://www.example.com"
        );
        field_eq!(
            table_iter.next().unwrap(),
            DATE,
            "Mon, 21 Oct 2013 20:13:21 GMT"
        );
        field_eq!(table_iter.next().unwrap(), CACHE_CONTROL, "private");
        field_eq!(table_iter.next().unwrap(), PSEUDO_STATUS, "302");
        assert!(table_iter.next().is_none());

        assert_eq!(table.size(), 222);
        assert_eq!(map.get(standard::PSEUDO_STATUS).unwrap(), &b"302"[..]);
        assert_eq!(map.get(standard::CACHE_CONTROL).unwrap(), &b"private"[..]);
        assert_eq!(
            map.get(standard::DATE).unwrap(),
            &b"Mon, 21 Oct 2013 20:13:21 GMT"[..]
        );
        assert_eq!(
            map.get(standard::LOCATION).unwrap(),
            &b"https://www.example.com"[..]
        );
    }

    // second response
    {
        let mut map = HeaderMap::new();
        let mut bytes = Bytes::copy_from_slice(res2);

        while !bytes.is_empty() {
            let field = table.decode(&mut bytes, &mut write_buffer).unwrap();
            map.try_append_field(field.into_owned()).unwrap();
        }

        let mut table_iter = table.fields().iter();
        field_eq!(table_iter.next().unwrap(), PSEUDO_STATUS, "307");
        field_eq!(
            table_iter.next().unwrap(),
            LOCATION,
            "https://www.example.com"
        );
        field_eq!(
            table_iter.next().unwrap(),
            DATE,
            "Mon, 21 Oct 2013 20:13:21 GMT"
        );
        field_eq!(table_iter.next().unwrap(), CACHE_CONTROL, "private");

        assert!(table_iter.next().is_none());
        assert_eq!(table.size(), 222);
        assert_eq!(map.get(standard::PSEUDO_STATUS).unwrap(), &b"307"[..]);
        assert_eq!(map.get(standard::CACHE_CONTROL).unwrap(), &b"private"[..]);
        assert_eq!(
            map.get(standard::DATE).unwrap(),
            &b"Mon, 21 Oct 2013 20:13:21 GMT"[..]
        );
        assert_eq!(
            map.get(standard::LOCATION).unwrap(),
            &b"https://www.example.com"[..]
        );
    }

    // third response
    {
        let mut map = HeaderMap::new();
        let mut bytes = Bytes::copy_from_slice(res3);

        while !bytes.is_empty() {
            let field = table.decode(&mut bytes, &mut write_buffer).unwrap();
            map.try_append_field(field.into_owned()).unwrap();
        }

        let mut table_iter = table.fields().iter();
        field_eq!(
            table_iter.next().unwrap(),
            SET_COOKIE,
            "foo=ASDJKHQKBZXOQWEOPIUAXQWEOIU; max-age=3600; version=1"
        );
        field_eq!(table_iter.next().unwrap(), CONTENT_ENCODING, "gzip");
        field_eq!(
            table_iter.next().unwrap(),
            DATE,
            "Mon, 21 Oct 2013 20:13:22 GMT"
        );
        assert!(table_iter.next().is_none());

        assert_eq!(table.size(), 215);
        assert_eq!(map.get(standard::PSEUDO_STATUS).unwrap(), &b"200"[..]);
        assert_eq!(map.get(standard::CACHE_CONTROL).unwrap(), &b"private"[..]);
        assert_eq!(
            map.get(standard::DATE).unwrap(),
            &b"Mon, 21 Oct 2013 20:13:22 GMT"[..]
        );
        assert_eq!(
            map.get(standard::LOCATION).unwrap(),
            &b"https://www.example.com"[..]
        );
        assert_eq!(map.get(standard::CONTENT_ENCODING).unwrap(), &b"gzip"[..]);
        assert_eq!(
            map.get(standard::SET_COOKIE).unwrap(),
            &b"foo=ASDJKHQKBZXOQWEOPIUAXQWEOIU; max-age=3600; version=1"[..]
        );
    }
}
