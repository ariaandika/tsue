use tcio::bytes::Bytes;

use super::path::Path;


#[test]
fn test_parse_path() {
    let bytes = Bytes::copy_from_slice(b"/users/all");
    let path = Path::parse(bytes).unwrap();
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), None);
}

#[test]
fn test_parse_path_query() {
    let bytes = Bytes::copy_from_slice(b"/users/all?query=1");
    let path = Path::parse(bytes).unwrap();
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), Some("query=1"));
}

#[test]
fn test_parse_path_query_empty() {
    let bytes = Bytes::copy_from_slice(b"/users/all?");
    let path = Path::parse(bytes).unwrap();
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), None);
}

#[test]
fn test_parse_path_empty_query() {
    let bytes = Bytes::copy_from_slice(b"?query=1");
    let path = Path::parse(bytes).unwrap();
    assert_eq!(path.path(), "/");
    assert_eq!(path.query(), Some("query=1"));
}

#[test]
fn test_parse_path_fragment() {
    let bytes = Bytes::copy_from_slice(b"/users/all#science");
    let path = Path::parse(bytes).unwrap();
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), None);
}

#[test]
fn test_parse_path_query_fragment() {
    let bytes = Bytes::copy_from_slice(b"/users/all?query=1#science");
    let path = Path::parse(bytes).unwrap();
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), Some("query=1"));
}

#[test]
fn test_parse_path_query_fragment_long() {
    let bytes = Bytes::copy_from_slice(b"/users/all?query=1&filter=trends#additional-section-4");
    let path = Path::parse(bytes).unwrap();
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), Some("query=1&filter=trends"));
}

#[test]
fn test_parse_path_partial_ascii() {
    let bytes = Bytes::copy_from_slice(b"/users/all?query=1&filter=trends#\xFF\xFF");
    let path = Path::parse(bytes).unwrap();
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), Some("query=1&filter=trends"));
}

#[test]
fn test_parse_path_non_ascii() {
    let bytes = Bytes::copy_from_slice(b"/users/all?query=1&filt\xFF\xFF");
    assert!(Path::parse(bytes).is_err());
}

