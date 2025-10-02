use crate::uri::{Scheme, Authority, Path};

macro_rules! assert_authority {
    (#[rest($id:ident)] $($m:ident())*, $ok:expr; $($tt:tt)*) => {
        $(assert_eq!($id.$m(), $ok, concat!("`",stringify!($m),"()`"));)*
        assert_authority!(#[rest($id)]$($tt)*);
    };
    (#[rest($id:ident)]) => { };
    ($parse:ident($input:expr); $($tt:tt)*) => {
        let ok = Authority::$parse($input).unwrap();
        assert_authority!(#[rest(ok)]$($tt)*);
    };
}

macro_rules! assert_path {
    (#[rest($id:ident)] $($m:ident())*, $ok:expr; $($tt:tt)*) => {
        $(assert_eq!($id.$m(), $ok, concat!("`",stringify!($m),"()`"));)*
        assert_path!(#[rest($id)]$($tt)*);
    };
    (#[rest($id:ident)]) => { };
    ($parse:ident($input:expr); $($tt:tt)*) => {
        let ok = Path::$parse($input).unwrap();
        assert_path!(#[rest(ok)]$($tt)*);
    };
}

#[test]
pub fn test_scheme() {
    assert!(Scheme::parse("http").is_ok());
    assert!(Scheme::parse("ftp").is_ok());
    assert!(Scheme::parse("postgresql").is_ok());

    assert!(Scheme::parse("http:").is_err());
    assert!(Scheme::parse("p\0ostgresql").is_err());
    assert!(Scheme::parse("postgresql\0").is_err());
}

#[test]
pub fn test_authority() {
    assert_authority! {
        parse("");
        host() hostname(), "";
        userinfo() port(), None;
    }
    assert_authority! {
        parse("example.com");
        host() hostname(), "example.com";
        userinfo() port(), None;
    }
    assert_authority! {
        parse("user:pass@example.com");
        userinfo(), Some("user:pass");
        host() hostname(), "example.com";
        port(), None;
    }
    assert_authority! {
        parse("example.com:443");
        userinfo(), None;
        host(), "example.com:443";
        hostname(), "example.com";
        port(), Some(443);
    }
    assert_authority! {
        parse("user:pass@example.com:443");
        userinfo(), Some("user:pass");
        host(), "example.com:443";
        hostname(), "example.com";
        port(), Some(443);
    }

    // note that currently the exact syntax of ipv6 is not validated

    assert_authority! {
        parse("[a2f::1]");
        host() hostname(), "[a2f::1]";
        userinfo() port(), None;
    }
    assert_authority! {
        parse("user:pass@[a2f::1]");
        userinfo(), Some("user:pass");
        host() hostname(), "[a2f::1]";
        port(), None;
    }
    assert_authority! {
        parse("[a2f::1]:443");
        userinfo(), None;
        host(), "[a2f::1]:443";
        hostname(), "[a2f::1]";
        port(), Some(443);
    }
    assert_authority! {
        parse("user:pass@[a2f::1]:443");
        userinfo(), Some("user:pass");
        host(), "[a2f::1]:443";
        hostname(), "[a2f::1]";
        port(), Some(443);
    }
}

#[test]
fn test_path() {
    assert_path! {
        parse("");
        path_and_query() path(), "";
        query(), None;
    }
    assert_path! {
        parse("/users/all");
        path_and_query() path(), "/users/all";
        query(), None;
    }
    assert_path! {
        parse("/users/all?");
        path_and_query(), "/users/all?";
        path(), "/users/all";
        query(), Some("");
    }
    assert_path! {
        parse("/users/all?page=420");
        path_and_query(), "/users/all?page=420";
        path(), "/users/all";
        query(), Some("page=420");
    }

    // fragment are trimmed

    assert_path! {
        parse("/users/all#section-443");
        path_and_query() path(), "/users/all";
        query(), None;
    }
    assert_path! {
        parse("/users/all?#section-443");
        path_and_query(), "/users/all?";
        path(), "/users/all";
        query(), Some("");
    }
    assert_path! {
        parse("/users/all?page=420#section-443");
        path_and_query(), "/users/all?page=420";
        path(), "/users/all";
        query(), Some("page=420");
    }
}

