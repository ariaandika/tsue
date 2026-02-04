use crate::headers::{AsHeaderName, IntoHeaderName};
use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue};
use crate::headers::standard as s;

#[test]
#[allow(clippy::borrow_interior_mutable_const)]
#[allow(clippy::declare_interior_mutable_const)]
fn test_header_map() {
    fn _is_dyn_compat(_: &dyn AsHeaderName) { }
    fn _is_dyn_compat2(_: &dyn IntoHeaderName) { }
    const fn is_send_sync<T: Send + Sync>() { }
    is_send_sync::<HeaderMap>();
    is_send_sync::<HeaderName>();
    is_send_sync::<HeaderValue>();
    is_send_sync::<HeaderField>();

    const FOO: HeaderValue = HeaderValue::from_static(b"FOO");

    let mut map = HeaderMap::new();

    assert!(map.insert(s::DATE, FOO).is_none());
    assert!(map.contains_key(s::DATE));

    let field = map.insert(s::DATE, FOO).unwrap();
    assert!(map.contains_key(s::DATE));
    assert_eq!(field.into_parts(), (s::DATE, FOO));

    assert!(map.insert(s::AGE, FOO).is_none());
    assert!(map.insert(s::HOST, FOO).is_none());
    assert!(map.insert(s::ACCEPT, FOO).is_none());
    assert!(map.insert(s::TE, FOO).is_none());

    let len = map.len();

    map.append(s::DATE, FOO);
    assert!(map.contains_key(s::DATE));

    assert_eq!(map.len(), len + 1);

    let mut fields = map.get_all(&s::DATE);
    assert_eq!(fields.next(), Some(&FOO));
    assert_eq!(fields.next(), Some(&FOO));
    assert!(fields.next().is_none());

    let mut i = 0;
    for field in &map {
        assert!(matches!(field.name().as_str(), "date" | "age" | "host" | "accept" | "te"));
        i += 1;
    }
    assert_eq!(map.len(), i);

    let field = map.remove(s::HOST).unwrap();
    assert!(!map.contains_key(s::HOST));
    assert_eq!(field.into_parts(), (s::HOST, FOO));

    let field = map.remove(s::DATE).unwrap();
    assert!(map.contains_key(s::DATE));
    assert_eq!(field.into_parts(), (s::DATE, FOO));
}
