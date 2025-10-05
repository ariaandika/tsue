use super::{Authority, HttpUri, Path, Scheme};

#[test]
pub fn test_scheme() {
    assert!(Scheme::from_slice("http").is_ok());
    assert!(Scheme::from_slice("ftp").is_ok());
    assert!(Scheme::from_slice("postgresql").is_ok());

    assert!(Scheme::from_slice("http:").is_err());
    assert!(Scheme::from_slice("p\0ostgresql").is_err());
    assert!(Scheme::from_slice("postgresql\0").is_err());
}

#[test]
pub fn test_authority() {
    let auth = Authority::from_slice("").unwrap();
    assert_eq!(auth.host(), "");
    assert_eq!(auth.hostname(), "",);
    assert_eq!(auth.userinfo(), None);
    assert_eq!(auth.port(), None);

    let auth = Authority::from_slice("example.com").unwrap();
    assert_eq!(auth.host(), "example.com");
    assert_eq!(auth.hostname(), "example.com");
    assert_eq!(auth.userinfo(), None);
    assert_eq!(auth.port(), None);

    let auth = Authority::from_slice("user:pass@example.com").unwrap();
    assert_eq!(auth.userinfo(), Some("user:pass"));
    assert_eq!(auth.host(), "example.com");
    assert_eq!(auth.hostname(), "example.com");
    assert_eq!(auth.port(), None);

    let auth = Authority::from_slice("example.com:443").unwrap();
    assert_eq!(auth.userinfo(), None);
    assert_eq!(auth.host(), "example.com:443");
    assert_eq!(auth.hostname(), "example.com");
    assert_eq!(auth.port(), Some(443));

    let auth = Authority::from_slice("user:pass@example.com:443").unwrap();
    assert_eq!(auth.userinfo(), Some("user:pass"));
    assert_eq!(auth.host(), "example.com:443");
    assert_eq!(auth.hostname(), "example.com");
    assert_eq!(auth.port(), Some(443));

    // note that currently the exact syntax of ipv6 is not validated

    let auth = Authority::from_slice("[a2f::1]").unwrap();
    assert_eq!(auth.host(), "[a2f::1]",);
    assert_eq!(auth.hostname(), "[a2f::1]");
    assert_eq!(auth.userinfo(), None);
    assert_eq!(auth.port(), None);

    let auth = Authority::from_slice("user:pass@[a2f::1]").unwrap();
    assert_eq!(auth.userinfo(), Some("user:pass"));
    assert_eq!(auth.host(), "[a2f::1]",);
    assert_eq!(auth.hostname(), "[a2f::1]");
    assert_eq!(auth.port(), None);

    let auth = Authority::from_slice("[a2f::1]:443").unwrap();
    assert_eq!(auth.userinfo(), None);
    assert_eq!(auth.host(), "[a2f::1]:443");
    assert_eq!(auth.hostname(), "[a2f::1]");
    assert_eq!(auth.port(), Some(443));

    let auth = Authority::from_slice("user:pass@[a2f::1]:443").unwrap();
    assert_eq!(auth.userinfo(), Some("user:pass"));
    assert_eq!(auth.host(), "[a2f::1]:443");
    assert_eq!(auth.hostname(), "[a2f::1]");
    assert_eq!(auth.port(), Some(443));
}

#[test]
fn test_path() {
    let path = Path::from_slice("").unwrap();
    assert_eq!(path.path_and_query(), "");
    assert_eq!(path.path(), "");
    assert_eq!(path.query(), None);

    let path = Path::from_slice("/users/all").unwrap();
    assert_eq!(path.path_and_query(), "/users/all");
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), None);

    let path = Path::from_slice("/users/all?").unwrap();
    assert_eq!(path.path_and_query(), "/users/all?");
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), Some(""));

    let path = Path::from_slice("/users/all?page=420").unwrap();
    assert_eq!(path.path_and_query(), "/users/all?page=420");
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), Some("page=420"));

    // fragment are trimmed

    let path = Path::from_slice("/users/all#section-443").unwrap();
    assert_eq!(path.path_and_query(), "/users/all");
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), None);

    let path = Path::from_slice("/users/all?#section-443").unwrap();
    assert_eq!(path.path_and_query(), "/users/all?");
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), Some(""));

    let path = Path::from_slice("/users/all?page=420#section-443").unwrap();
    assert_eq!(path.path_and_query(), "/users/all?page=420");
    assert_eq!(path.path(), "/users/all");
    assert_eq!(path.query(), Some("page=420"));
}

#[test]
fn test_http_uri() {
    let ok = HttpUri::from_slice("http://example.com/users/all?page=420#section-443").unwrap();
    assert!(!ok.is_https());
    assert_eq!(ok.authority(), "example.com");
    assert_eq!(ok.path(), "/users/all");
}
