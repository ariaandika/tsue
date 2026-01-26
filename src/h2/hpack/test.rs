use tcio::bytes::{Bytes, BytesMut};

use crate::h2::hpack::Table;
use crate::headers::{HeaderMap, HeaderName, standard};

#[test]
fn test_hpack_appendix_c3() {
    let mut table = Table::new();
    let mut write_buffer = BytesMut::new();

    {
        let req1 = Bytes::copy_from_slice(&[
            0x82, 0x86, 0x84, 0x41, 0x0f, 0x77, 0x77, 0x77,
            0x2e, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65,
            0x2e, 0x63, 0x6f, 0x6d,
        ]);
        let mut map = HeaderMap::new();
        table.decode_block(req1, &mut map, &mut write_buffer).unwrap();

        let mut fields = table.fields().iter();
        let field = fields.next().unwrap();
        assert_eq!(field.name, standard::PSEUDO_AUTHORITY);
        assert_eq!(field.value.as_str(), "www.example.com");
        assert!(fields.next().is_none());

        assert_eq!(table.size(), 57);

        assert_eq!(map.get(standard::PSEUDO_METHOD).unwrap(), &b"GET"[..]);
        assert_eq!(map.get(standard::PSEUDO_SCHEME).unwrap(), &b"http"[..]);
        assert_eq!(map.get(standard::PSEUDO_PATH).unwrap(), &b"/"[..]);
        assert_eq!(map.get(standard::PSEUDO_AUTHORITY).unwrap(), &b"www.example.com"[..]);
    }

    {
        let req2 = Bytes::copy_from_slice(&[
            0x82, 0x86, 0x84, 0xbe, 0x58, 0x08, 0x6e, 0x6f,
            0x2d, 0x63, 0x61, 0x63, 0x68, 0x65,
        ]);
        let mut map = HeaderMap::new();
        table
            .decode_block(req2, &mut map, &mut write_buffer)
            .unwrap();

        let mut fields = table.fields().iter();

        let field = fields.next().unwrap();
        assert_eq!(field.name, standard::CACHE_CONTROL);
        assert_eq!(field.value.as_str(), "no-cache");

        let field = fields.next().unwrap();
        assert_eq!(field.name, standard::PSEUDO_AUTHORITY);
        assert_eq!(field.value.as_str(), "www.example.com");

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
        // assert_eq!(
        //     map.get(HeaderName::from_static(b"custom-key")).unwrap(),
        //     &b"custom-value"[..]
        // );
    }

    {
        let req3 = Bytes::copy_from_slice(&[
            0x82, 0x87, 0x85, 0xbf, 0x40, 0x0a, 0x63, 0x75,
            0x73, 0x74, 0x6f, 0x6d, 0x2d, 0x6b, 0x65, 0x79,
            0x0c, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d, 0x2d,
            0x76, 0x61, 0x6c, 0x75, 0x65,
        ]);
        let mut map = HeaderMap::new();
        table
            .decode_block(req3, &mut map, &mut write_buffer)
            .unwrap();

        // dynamic table
        let mut fields = table.fields().iter();

        let field = fields.next().unwrap();
        assert_eq!(field.name, HeaderName::from_static(b"custom-key"));
        assert_eq!(field.value.as_str(), "custom-value");

        let field = fields.next().unwrap();
        assert_eq!(field.name, standard::CACHE_CONTROL);
        assert_eq!(field.value.as_str(), "no-cache");

        let field = fields.next().unwrap();
        assert_eq!(field.name, standard::PSEUDO_AUTHORITY);
        assert_eq!(field.value.as_str(), "www.example.com");

        assert!(fields.next().is_none());
        assert_eq!(table.size(), 164);

        // decoded header map

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

