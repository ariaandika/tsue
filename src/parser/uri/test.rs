
#[test]
fn test_uri_parse() {
    use super::Uri;

    macro_rules! test {
        {
            #[error]
            $uri:literal
        } => {
            assert!(Uri::try_copy_from($uri).is_err());
        };
        {
            $uri:literal;
            $scheme:literal;
            $(#[$no:tt])?
            $auth:expr => {
                $uinfo:expr;
                $host:expr;
                $hostname:expr;
                $port:expr;
            };
            $pq:literal;
            $path:literal;
            $query:expr;
        } => {
            let uri = Uri::try_copy_from($uri).unwrap();
            dbg!(&uri);
            assert_eq!(uri.scheme(), $scheme, "scheme");
            assert_eq!(uri.authority_str(), $auth, "authority");
            assert_eq!(uri.userinfo(), $uinfo, "userinfo");
            assert_eq!(uri.host(), $host, "host");
            assert_eq!(uri.hostname(), $hostname, "hostname");
            assert_eq!(uri.port(), $port, "port");
            assert_eq!(uri.path_and_query(), $pq, "path and query");
            assert_eq!(uri.path(), $path, "path");
            assert_eq!(uri.query(), $query, "query");
        };
        {
            $uri:literal;
            $scheme:literal;
            $auth:expr => { };
            $pq:literal;
            $path:literal;
            $query:expr;
        } => {
            test! {
                $uri;
                $scheme;
                $auth => { None; None; None; None; };
                $pq;
                $path;
                $query;
            }
        }
    }

    // path only

    test! {
        b"/users/all?filter=favorite&page=4#additional-section-4";
        "";
        None => { };
        "/users/all?filter=favorite&page=4";
        "/users/all";
        Some("filter=favorite&page=4");
    }

    // general form

    test! {
        b"http://user:pass@example.com:443/users/all?filter=favorite&page=4#additional-section-4";
        "http";
        Some("user:pass@example.com:443") => {
            Some("user:pass");
            Some("example.com:443");
            Some("example.com");
            Some(443);
        };
        "/users/all?filter=favorite&page=4";
        "/users/all";
        Some("filter=favorite&page=4");
    }

    test! {
        b"file:///home/users/downloads";
        "file";
        None => { };
        "/home/users/downloads";
        "/home/users/downloads";
        None;
    }

    test! {
        b"/users/all?filter=favorite&page=4#additional-section-4";
        "";
        None => { };
        "/users/all?filter=favorite&page=4";
        "/users/all";
        Some("filter=favorite&page=4");
    }

    // errors

    test!(#[error] b"");

    test!(#[error] b"http://exa mple.com/path");
    // test!(#[error] b"http://example.com:80a/path");
    // test!(#[error] b"http://user@pass:word@example.com");
    // test!(#[error] b"http://example.com:999999/path");
}

