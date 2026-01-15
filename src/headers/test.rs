use crate::headers::{AsHeaderName, IntoHeaderName};
use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue};

const fn is_send_sync<T: Send + Sync>() { }
const _: () = {
    is_send_sync::<HeaderMap>();
    is_send_sync::<HeaderName>();
    is_send_sync::<HeaderValue>();
    is_send_sync::<HeaderField>();
    fn _is_dyn_compat(_: &dyn AsHeaderName) { }
    fn _is_dyn_compat2(_: &dyn IntoHeaderName) { }
};

#[test]
fn header_map() {
    let mut map = HeaderMap::new();

    map.insert("content-type", HeaderValue::from_string("FOO"));
    assert!(map.contains_key("content-type"));

    let cap = map.capacity();

    assert!(map.insert("accept", HeaderValue::from_string("BAR")).is_none());
    assert!(map.insert("content-length", HeaderValue::from_string("LEN")).is_none());
    assert!(map.insert("host", HeaderValue::from_string("BAR")).is_none());
    assert!(map.insert("date", HeaderValue::from_string("BAR")).is_none());
    assert!(map.insert("referer", HeaderValue::from_string("BAR")).is_none());
    assert!(map.insert("rim", HeaderValue::from_string("BAR")).is_none());

    assert!(map.contains_key("content-type"));
    assert!(map.contains_key("accept"));
    assert!(map.contains_key("content-length"));
    assert!(map.contains_key("host"));
    assert!(map.contains_key("date"));
    assert!(map.contains_key("referer"));
    assert!(map.contains_key("rim"));

    // Insert Allocate

    assert!(map.insert("lea", HeaderValue::from_string("BAR")).is_none());

    // assert_ne!(ptr, map.fields.as_ptr());
    assert_ne!(cap, map.capacity());

    assert!(map.contains_key("content-type"));
    assert!(map.contains_key("accept"));
    assert!(map.contains_key("content-length"));
    assert!(map.contains_key("host"));
    assert!(map.contains_key("date"));
    assert!(map.contains_key("referer"));
    assert!(map.contains_key("rim"));
    assert!(map.contains_key("lea"));

    // Insert Multi

    map.append("content-length", HeaderValue::from_string("BAR"));

    assert!(map.contains_key("content-length"));
    assert!(map.contains_key("host"));
    assert!(map.contains_key("date"));
    assert!(map.contains_key("referer"));
    assert!(map.contains_key("rim"));

    let mut all = map.get_all("content-length");
    assert!(matches!(all.next(), Some(v) if matches!(v.as_str(),"LEN")));
    assert!(matches!(all.next(), Some(v) if matches!(v.as_str(),"BAR")));
    assert!(all.next().is_none());

    // Remove accept

    assert!(map.remove("accept").is_some());
    assert!(map.contains_key("content-type"));
    assert!(map.contains_key("content-length"));
    assert!(map.contains_key("host"));
    assert!(map.contains_key("date"));
    assert!(map.contains_key("referer"));
    assert!(map.contains_key("rim"));
    assert!(map.contains_key("lea"));

    // Remove lea

    assert!(map.remove("lea").is_some());
    assert!(map.contains_key("content-type"));
    assert!(map.contains_key("content-length"));
    assert!(map.contains_key("host"));
    assert!(map.contains_key("date"));
    assert!(map.contains_key("referer"));
    assert!(map.contains_key("rim"));

    assert!(map.remove("content-length").is_some());

    // Clear

    map.clear();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
    assert!(!map.contains_key("content-type"));
    assert!(!map.contains_key("host"));
    assert!(!map.contains_key("date"));
    assert!(!map.contains_key("referer"));
    assert!(!map.contains_key("rim"));
}

// const fn mask(cap: usize, value: u32) -> u32 {
//     value & (cap as u32 - 1)
// }
//
// pub struct MapDbg<'a>(pub &'a HeaderMap);
// pub struct FieldsDbg<'a>(pub &'a HeaderMap);
//
// impl std::fmt::Debug for MapDbg<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let mut m = f.debug_struct("HeaderMap");
//         m.field("len", &self.0.len());
//         m.field("cap", &self.0.capacity());
//         m.field("fields", &FieldsDbg(self.0));
//         m.finish()
//     }
// }
//
// impl std::fmt::Debug for FieldsDbg<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let mut m = f.debug_list();
//         for field in self.0.fields() {
//             match field {
//                 Some(field) => m.entry(&format_args!(
//                     "{}({}->{}): {:?}",
//                     field.name().as_str(),
//                     field.cached_hash(),
//                     mask(self.0.capacity(), field.cached_hash()),
//                     field.value(),
//                 )),
//                 None => m.entry(&None::<()>),
//             };
//         }
//         m.finish()
//     }
// }
