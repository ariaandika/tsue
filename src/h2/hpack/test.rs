use tcio::bytes::{Bytes, BytesMut};

use crate::h2::hpack::Table;
use crate::headers::{HeaderMap, HeaderName, standard};

macro_rules! test_hpack {
    (
        [$($bits:literal),* $(,)?],
        |$table:ident,$bytes:ident,$write_buffer:ident|$body:expr
    ) => {
        let mut $table = Table::new();
        let mut $write_buffer = BytesMut::new();
        let mut $bytes = Bytes::copy_from_slice(&[$($bits),*]);
        $body
    };
    (
        [$($bits:literal),* $(,)?],
        |$map:ident, $bytes:ident|$body:expr
    ) => {{
        let mut $map = HeaderMap::new();
        let $bytes = Bytes::copy_from_slice(&[$($bits),*]);
        $body
    }};
}

macro_rules! field_eq {
    ($field:expr, $name:literal, $val:literal) => {{
        let f = $field;
        assert_eq!(f.name.as_str(), $name);
        assert_eq!(f.value.as_str(), $val);
    }};
    ($field:expr, $std:ident, $val:literal) => {{
        let f = $field;
        assert_eq!(f.name, standard::$std);
        assert_eq!(f.value.as_str(), $val);
    }};
}

#[test]
fn test_hpack_appendix_c2_1() {
    test_hpack! {
        [
            0x40, 0x0a, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d,
            0x2d, 0x6b, 0x65, 0x79, 0x0d, 0x63, 0x75, 0x73,
            0x74, 0x6f, 0x6d, 0x2d, 0x68, 0x65, 0x61, 0x64,
            0x65, 0x72,
        ],
        |table, bytes, write_buffer|{
            let field = table.decode_test(&mut bytes, &mut write_buffer).unwrap();
            field_eq!(field, "custom-key", "custom-header");

            assert_eq!(table.size(), 55);

            let ([entry], []) = table.fields().as_slices() else { unreachable!() };
            field_eq!(entry, "custom-key", "custom-header");
        }
    }
}

#[test]
fn test_hpack_appendix_c2_2() {
    test_hpack! {
        [
            0x04, 0x0c, 0x2f, 0x73, 0x61, 0x6d, 0x70, 0x6c,
            0x65, 0x2f, 0x70, 0x61, 0x74, 0x68,
        ],
        |table, bytes, write_buffer|{
            let field = table.decode_test(&mut bytes, &mut write_buffer).unwrap();
            field_eq!(field, PSEUDO_PATH, "/sample/path");

            assert!(table.fields().is_empty());
            assert_eq!(table.size(), 0);
        }
    }
}

#[test]
fn test_hpack_appendix_c3() {
    let mut table = Table::new();
    let mut write_buffer = BytesMut::new();

    // first request
    test_hpack! {
        [
            0x82, 0x86, 0x84, 0x41, 0x0f, 0x77, 0x77, 0x77,
            0x2e, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65,
            0x2e, 0x63, 0x6f, 0x6d,
        ],
        |map, bytes|{
            table.decode_block(bytes, &mut map, &mut write_buffer).unwrap();

            let mut fields = table.fields().iter();
            let field = fields.next().unwrap();
            field_eq!(field, PSEUDO_AUTHORITY, "www.example.com");
            assert!(fields.next().is_none());

            assert_eq!(table.size(), 57);

            assert_eq!(map.get(standard::PSEUDO_METHOD).unwrap(), &b"GET"[..]);
            assert_eq!(map.get(standard::PSEUDO_SCHEME).unwrap(), &b"http"[..]);
            assert_eq!(map.get(standard::PSEUDO_PATH).unwrap(), &b"/"[..]);
            assert_eq!(map.get(standard::PSEUDO_AUTHORITY).unwrap(), &b"www.example.com"[..]);
        }
    }

    // second request
    test_hpack! {
        [
            0x82, 0x86, 0x84, 0xbe, 0x58, 0x08, 0x6e, 0x6f,
            0x2d, 0x63, 0x61, 0x63, 0x68, 0x65,
        ],
        |map, bytes|{
            table.decode_block(bytes, &mut map, &mut write_buffer).unwrap();

            let mut fields = dbg!(table.fields()).iter();
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
            assert_eq!(
                map.get(standard::CACHE_CONTROL).unwrap(),
                &b"no-cache"[..]
            );
        }
    }

    // third request
    test_hpack! {
        [
            0x82, 0x87, 0x85, 0xbf, 0x40, 0x0a, 0x63, 0x75,
            0x73, 0x74, 0x6f, 0x6d, 0x2d, 0x6b, 0x65, 0x79,
            0x0c, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d, 0x2d,
            0x76, 0x61, 0x6c, 0x75, 0x65,
        ],
        |map, bytes|{
            table.decode_block(bytes, &mut map, &mut write_buffer).unwrap();

            let mut fields = dbg!(table.fields()).iter();
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
}

