use tcio::bytes::Buf;

use crate::uri::{Authority, Bytes, Host, HttpScheme, HttpUri, Path, Scheme, Uri, UriError};

impl Scheme {
    /// Validate scheme from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid scheme.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        if validate_scheme(bytes) {
            Self {
                value: Bytes::from_static(bytes),
            }
        } else {
            UriError::InvalidScheme.panic_const();
        }
    }

    /// Validate scheme from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid scheme.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        if validate_scheme(value.as_slice()) {
            Ok(Self { value })
        } else {
            Err(UriError::InvalidScheme)
        }
    }

    /// Validate scheme by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid scheme.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        if validate_scheme(bytes.as_ref()) {
            Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
            })
        } else {
            Err(UriError::InvalidScheme)
        }
    }
}

impl Authority {
    /// Validate authority from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid authority.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_authority(bytes) {
            Some([]) => Self {
                value: Bytes::from_static(bytes),
            },
            _ => UriError::InvalidAuthority.panic_const()
        }
    }

    /// Validate authority from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid authority.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        match validate_authority(value.as_slice()) {
            Some([]) => Ok(Self { value }),
            _ => Err(UriError::InvalidAuthority)
        }
    }

    /// Validate authority by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid authority.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match validate_authority(bytes.as_ref()) {
            Some([]) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
            }),
            _ => Err(UriError::InvalidAuthority),
        }
    }
}

impl Host {
    /// Validate host from static bytes.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid host.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_host(bytes) {
            Some([]) => Self { value: Bytes::from_static(bytes) },
            _ => UriError::InvalidAuthority.panic_const(),
        }
    }

    /// Validate host from [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid host.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let value = bytes.into();
        match validate_host(value.as_slice()) {
            Some([]) => Ok(Self { value }),
            _ => Err(UriError::InvalidAuthority),
        }
    }

    /// Validate host by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid host.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        match validate_host(bytes.as_ref()) {
            Some([]) => Ok(Self {
                value: Bytes::copy_from_slice(bytes.as_ref()),
            }),
            _ => Err(UriError::InvalidAuthority),
        }
    }
}

impl Path {
    pub(crate) const MAX_LEN: u16 = 8 * 1024;

    /// Validate path from static bytes.
    ///
    /// Path fragment is trimmed.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid path.
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        match validate_path(bytes) {
            Ok((query, slice)) => Self {
                value: Bytes::from_static(slice),
                query,
            },
            Err(err) => err.panic_const(),
        }
    }

    /// Validate path from [`Bytes`].
    ///
    /// Path fragment is trimmed.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid path.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        let mut bytes = bytes.into();
        let (query, slice) = validate_path(bytes.as_slice())?;
        bytes.truncate(slice.len());
        Ok(Self {
            value: bytes,
            query,
        })
    }

    /// Validate path by copying from slice of bytes.
    ///
    /// Path fragment is trimmed.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid path.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        let (query, slice) = validate_path(bytes.as_ref())?;
        let value = Bytes::copy_from_slice(slice);
        Ok(Self { value, query })
    }
}

impl Uri {
    /// Creates [`Uri`] from [`Scheme`], optionally [`Authority`], and [`Path`].
    #[inline]
    pub const fn from_parts(scheme: Scheme, authority: Option<Authority>, path: Path) -> Self {
        // TODO: check that path should be `path-abempty` ?
        // > [RFC3986#section-3] When authority is present, the path must either be empty or begin
        // > with a slash ("/") character

        Self {
            scheme,
            authority,
            path,
        }
    }

    /// Parse URI from [`Bytes`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use tsue::uri::Uri;
    /// let http = Uri::from_bytes("http://example.com/users/all").unwrap();
    /// assert_eq!(http.host(), Some("example.com"));
    /// assert_eq!(http.path(), "/users/all");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid URI.
    #[inline]
    pub fn from_bytes<B: Into<Bytes>>(bytes: B) -> Result<Self, UriError> {
        parse_uri(bytes.into())
    }

    /// Parse URI by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid URI.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        parse_uri(Bytes::copy_from_slice(bytes.as_ref()))
    }
}

impl HttpUri {
    /// Parse HTTP URI from [`Bytes`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tsue::uri::HttpUri;
    /// let http = HttpUri::from_bytes("http://example.com/users/all").unwrap();
    /// assert_eq!(http.host(), "example.com");
    /// assert_eq!(http.path(), "/users/all");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid HTTP URI.
    #[inline]
    pub fn from_bytes(bytes: impl Into<Bytes>) -> Result<Self, UriError> {
        parse_http(bytes.into())
    }

    /// Parse HTTP URI by copying from slice of bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the input is not a valid HTTP URI.
    #[inline]
    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Result<Self, UriError> {
        parse_http(Bytes::copy_from_slice(bytes.as_ref()))
    }
}

// ===== Validation =====

mod validate {
    /// pct-encoded = "%" HEXDIG HEXDIG
    pub const fn pct_encoded(byte: u8) -> bool {
        byte == b'%' || byte.is_ascii_hexdigit()
    }

    /// unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"
    pub const fn unreserved(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
    }

    /// sub-delims = "!" / "$" / "&" / "'" / "(" / ")"
    ///            / "*" / "+" / "," / ";" / "="
    pub const fn sub_delims(byte: u8) -> bool {
        matches!(
            byte,
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
        ) || byte.is_ascii_alphanumeric()
    }
}

// ===== Scheme =====

crate::matches::ascii_lookup_table! {
    /// scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    const fn is_scheme(byte: u8) -> bool {
        byte.is_ascii_alphanumeric()
        || matches!(byte, b'+' | b'-' | b'.')
    }
}

/// scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
const fn validate_scheme(mut bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    while let [byte, rest @ ..] = bytes {
        if !is_scheme(*byte) {
            return false;
        }
        bytes = rest
    }
    true
}

// ===== Authority =====

/// `authority = [ userinfo "@" ] host [ ":" port ]`
const fn validate_authority(bytes: &[u8]) -> Option<&[u8]> {
    let [prefix, ..] = bytes else {
        return Some(bytes);
    };

    let mut state = bytes;

    if *prefix == b'[' {
        let Some(rest) = validate_ip_literal(bytes) else {
            return None;
        };
        state = rest;
    } else {
        // fast path for empty authority in hier-part
        if is_path_delim(*prefix) {
            return Some(bytes);
        }

        // userinfo or host
        //
        // userinfo    = *( unreserved / pct-encoded / sub-delims / ":" )
        // IPv4address = dec-octet "." dec-octet "." dec-octet "." dec-octet
        // reg-name    = *( unreserved / pct-encoded / sub-delims )
        let mut delim = loop {
            let [byte, rest @ ..] = state else {
                // host only
                return Some(state);
            };
            state = rest;
            if !is_regname(*byte) {
                break *byte;
            }
        };

        let mut is_port_ok = true;

        while delim == b':' {
            // userinfo or port delimiter
            loop {
                let [byte, rest @ ..] = state else {
                    // without userinfo, with port
                    return if is_port_ok {
                        Some(state)
                    } else {
                        None
                    }
                };

                is_port_ok &= byte.is_ascii_digit();
                state = rest;

                if !is_regname(*byte) {
                    delim = *byte;
                    break;
                }
            }

            // port delimiter can only appear once
            is_port_ok = false;
        }

        if delim != b'@' {
            // host only, followed by other component
            return Some(state)
        }

        if let [prefix, ..] = state && *prefix == b'[' {
            match validate_ip_literal(state) {
                Some(rest) => state = rest,
                None => return None,
            }
        } else {
            loop {
                let [byte, rest @ ..] = state else {
                    // with userinfo, without port
                    return Some(state);
                };
                if !is_regname(*byte) {
                    break;
                }
                state = rest;
            }
        }
    }

    let Some((delim, mut state)) = state.split_first() else {
        return Some(state);
    };

    if *delim != b':' {
        // with userinfo, without port, followed by other component
        return Some(state)
    }

    loop {
        let [digit, rest @ ..] = state else {
            // with userinfo and port
            return Some(state);
        };
        if !digit.is_ascii_digit() {
            // with userinfo and port, followed by other component
            return Some(state);
        }
        state = rest;
    }
}

crate::matches::ascii_lookup_table! {
    /// reg-name = *( unreserved / pct-encoded / sub-delims )
    const fn is_regname(byte: u8) -> bool {
        validate::unreserved(byte)
        || validate::pct_encoded(byte)
        || validate::sub_delims(byte)
    }
}

/// host          = IP-literal / IPv4address / reg-name
/// IP-literal    = "[" ( IPv6address / IPvFuture  ) "]"
/// IPv4address   = dec-octet "." dec-octet "." dec-octet "." dec-octet
/// reg-name      = *( unreserved / pct-encoded / sub-delims )
///
/// dec-octet     = DIGIT                 ; 0-9
///               / %x31-39 DIGIT         ; 10-99
///               / "1" 2DIGIT            ; 100-199
///               / "2" %x30-34 DIGIT     ; 200-249
///               / "25" %x30-35          ; 250-255
const fn validate_host(bytes: &[u8]) -> Option<&[u8]> {
    let Some((prefix, mut state)) = bytes.split_first() else {
        return Some(bytes);
    };
    if *prefix == b'[' {
        match validate_ip_literal(bytes) {
            Some([]) => Some(bytes),
            _ => None,
        }
    } else {
        loop {
            let [byte, rest @ ..] = state else {
                return Some(bytes);
            };
            if !is_regname(*byte) {
                return Some(bytes);
            }
            state = rest;
        }
    }
}

crate::matches::ascii_lookup_table! {
    /// hex / ":" / "."
    const fn is_ipv6(byte: u8) -> bool {
        byte.is_ascii_hexdigit()
        || matches!(byte, b':' | b'.')
    }
}

crate::matches::ascii_lookup_table! {
    /// IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
    const fn is_ipvfuture(byte: u8) -> bool {
        validate::unreserved(byte)
        || validate::sub_delims(byte)
        || matches!(byte, b':')
    }
}

/// IP-literal    = "[" ( IPv6address / IPvFuture  ) "]"
/// IPv6address   =                            6( h16 ":" ) ls32
///               /                       "::" 5( h16 ":" ) ls32
///               / [               h16 ] "::" 4( h16 ":" ) ls32
///               / [ *1( h16 ":" ) h16 ] "::" 3( h16 ":" ) ls32
///               / [ *2( h16 ":" ) h16 ] "::" 2( h16 ":" ) ls32
///               / [ *3( h16 ":" ) h16 ] "::"    h16 ":"   ls32
///               / [ *4( h16 ":" ) h16 ] "::"              ls32
///               / [ *5( h16 ":" ) h16 ] "::"              h16
///               / [ *6( h16 ":" ) h16 ] "::"
/// IPvFuture     = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
///
/// h16           = 1*4HEXDIG
/// ls32          = ( h16 ":" h16 ) / IPv4address
/// IPv4address   = dec-octet "." dec-octet "." dec-octet "." dec-octet
/// dec-octet     = DIGIT                 ; 0-9
///               / %x31-39 DIGIT         ; 10-99
///               / "1" 2DIGIT            ; 100-199
///               / "2" %x30-34 DIGIT     ; 200-249
///               / "25" %x30-35          ; 250-255
const fn validate_ip_literal(bytes: &[u8]) -> Option<&[u8]> {
    let Some((b'[', mut state)) = bytes.split_first() else {
        unreachable!()
    };

    // IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
    let close_delim = if let [b'v', hexdig1, rest @ ..] = state {
        if !hexdig1.is_ascii_hexdigit() || rest.is_empty() {
            return None;
        }
        state = rest;

        while let [byte, rest @ ..] = state {
            state = rest;

            if !byte.is_ascii_hexdigit() {
                if *byte != b'.' {
                    return None;
                }
                break;
            }
        }

        if state.is_empty() {
            return None;
        }

        loop {
            let [byte, rest @ ..] = state else {
                return None;
            };
            state = rest;
            if !is_ipvfuture(*byte) {
                break *byte;
            }
        }
    } else {
        // FEAT: validate ipv6
        loop {
            let [byte, rest @ ..] = state else {
                return None;
            };
            state = rest;
            if !is_ipv6(*byte) {
                break *byte;
            }
        }
    };

    if close_delim == b']' {
        Some(state)
    } else {
        None
    }
}

// ===== Path =====

const fn is_path_delim(byte: u8) -> bool {
    matches!(byte, b'/' | b'?' | b'#')
}

crate::matches::ascii_lookup_table! {
    /// pchar = unreserved / pct-encoded / sub-delims / ":" / "@"
    const fn is_pchar(byte: u8) -> bool {
        validate::unreserved(byte)
        || validate::pct_encoded(byte)
        || validate::sub_delims(byte)
        || matches!(byte, b':' | b'@')
    }
}

crate::matches::ascii_lookup_table! {
    /// query = *( pchar / "/" / "?" )
    const fn is_query(byte: u8) -> bool {
        is_pchar(byte)
        || matches!(byte, b'/' | b'?')
    }
}

/// This allows for query component.
///
/// path          = path-abempty    ; begins with "/" or is empty
///               / path-absolute   ; begins with "/" but not "//"
///               / path-noscheme   ; begins with a non-colon segment
///               / path-rootless   ; begins with a segment
///               / path-empty      ; zero characters
///
/// path-abempty  = *( "/" segment )
/// path-absolute = "/" [ segment-nz *( "/" segment ) ]
/// path-noscheme = segment-nz-nc *( "/" segment )
/// path-rootless = segment-nz *( "/" segment )
/// path-empty    = 0<pchar>
///
/// segment       = *pchar
/// segment-nz    = 1*pchar
/// segment-nz-nc = 1*( unreserved / pct-encoded / sub-delims / "@" )
///               ; non-zero-length segment without any colon ":"
///
/// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
const fn validate_path(mut bytes: &[u8]) -> Result<(u16, &[u8]), UriError> {
    if bytes.is_empty() {
        return Ok((0, &[]));
    }

    let mut query = bytes.len() as u16;
    let mut frag = bytes.len();

    while let [byte, rest @ ..] = bytes {
        if !is_pchar(*byte) {
            if *byte == b'?' {
                bytes = rest;
                query = query - rest.len() as u16 - 1;
                break;
            } else if *byte == b'#' {
                frag = frag - rest.len() - 1;
                query = frag as u16;
                bytes = &[];
                break;
            } else {
                return Err(UriError::InvalidPath);
            }
        }
        bytes = rest;
    }

    while let [byte, rest @ ..] = bytes {
        if !is_query(*byte) {
            if *byte != b'#' {
                return Err(UriError::InvalidPath);
            }
            frag = frag - rest.len() - 1;
            break;
        }
        bytes = rest;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes.as_ptr(), frag) };

    Ok((query, slice))
}

// ===== Uri =====

fn parse_uri(mut bytes: Bytes) -> Result<Uri, UriError> {
    let mut state = bytes.as_slice();

    loop {
        let [prefix, rest @ ..] = state else {
            return Err(UriError::InvalidScheme);
        };
        if !is_scheme(*prefix) {
            if *prefix != b':' {
                return Err(UriError::InvalidScheme);
            }
            break;
        }
        state = rest;
    }
    let scheme = Scheme {
        value: unsafe {
            bytes.split_to_unchecked(state.as_ptr().offset_from_unsigned(bytes.as_ptr()))
        },
    };
    unsafe { bytes.advance_unchecked(1) };

    let authority = if bytes.starts_with(b"//") {
        bytes.advance(2);

        let authority = match find_path_delim(bytes.as_slice()) {
            Some(at) => unsafe { bytes.split_to_unchecked(at) },
            None => std::mem::take(&mut bytes),
        };

        if authority.is_empty() {
            None
        } else {
            Some(Authority::from_bytes(authority)?)
        }
    } else {
        None
    };

    let path = Path::from_bytes(bytes)?;

    Ok(Uri {
        scheme,
        authority,
        path,
    })
}

/*

/// URI         = scheme ":" hier-part [ "?" query ] [ "#" fragment ]
/// hier-part   = "//" authority path-abempty
///             / path-absolute
///             / path-rootless
///             / path-empty
const fn parse_uri2(bytes: &[u8]) -> Result<UriParts, UriError> {
    let mut state = bytes;

    let col = loop {
        let [scheme, rest @ ..] = state else {
            return Err(UriError::InvalidScheme);
        };
        state = rest;
        if !is_scheme(*scheme) {
            if *scheme != b':' {
                return Err(UriError::InvalidScheme);
            }
            break std::ptr::NonNull::from_ref(scheme);
        }
    };

    let auth = if let Some((b"//", rest)) = state.split_first_chunk() {
        let Some(rest) = validate_authority(rest) else {
            return Err(UriError::InvalidAuthority)
        };
        if let Some(delim) = rest.first() && !is_path_delim(*delim) {
            return Err(UriError::InvalidAuthority);
        }
        state = rest;
        state.as_ptr()
    } else {
        std::ptr::null()
    };

    let (query, path) = match validate_path(state) {
        Ok(ok) => ok,
        Err(err) => return Err(err),
    };

    Ok(UriParts {
        col,
        auth,
        path: path.as_ptr(),
        query,
    })
}

struct UriParts {
    col: std::ptr::NonNull<u8>,
    auth: *const u8,
    path: *const u8,
    query: u16,
}

*/

/// ```not_rust
/// http-URI = "http" "://" authority path-abempty [ "?" query ]
/// https-URI = "https" "://" authority path-abempty [ "?" query ]
/// ```
fn parse_http(mut bytes: Bytes) -> Result<HttpUri, UriError> {
    let scheme = if bytes.starts_with(b"http://") {
        HttpScheme::HTTP
    } else if bytes.starts_with(b"https://") {
        HttpScheme::HTTPS
    } else {
        return Err(UriError::InvalidScheme);
    };

    bytes.advance(5 + 2 + scheme.is_https() as usize);

    let host = match find_path_delim(bytes.as_slice()) {
        Some(at) => unsafe { bytes.split_to_unchecked(at) },
        None => std::mem::take(&mut bytes),
    };

    // > A sender MUST NOT generate an "http" URI with an empty host identifier.
    if host.is_empty() {
        return Err(UriError::InvalidAuthority);
    }

    let host = Host::from_bytes(host)?;
    let path = Path::from_slice(bytes)?;

    Ok(HttpUri::from_parts(scheme, host, path))
}

// ===== SWAR =====

const BLOCK: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
const SLASH: usize = usize::from_ne_bytes([b'/'; BLOCK]);
const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);

const fn find_path_delim(bytes: &[u8]) -> Option<usize> {
    let mut state: &[u8] = bytes;

    while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);

        // '/'
        let is_slash = (block ^ SLASH).wrapping_sub(LSB);
        // '?'
        let is_qs = (block ^ QS).wrapping_sub(LSB);
        // '#'
        let is_hash = (block ^ HASH).wrapping_sub(LSB);

        let result = (is_slash | is_qs | is_hash | block) & MSB;

        if result != 0 {
            let nth = (result.trailing_zeros() / 8) as usize;
            return unsafe {
                Some(state.as_ptr().offset_from_unsigned(bytes.as_ptr()) + nth)
            }
        }

        state = rest;
    }

    loop {
        let [byte, rest @ ..] = state else {
            return None;
        };

        if matches!(byte, b'/' | b'?' | b'#') || !byte.is_ascii() {
            return unsafe {
                Some(state.as_ptr().offset_from_unsigned(bytes.as_ptr()))
            }
        }

        state = rest;
    }
}

// const fn find_path_delim2(bytes: &[u8]) -> usize {
//     let mut state: &[u8] = bytes;
//
//     while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
//         let block = usize::from_ne_bytes(*chunk);
//
//         // '/'
//         let is_slash = (block ^ SLASH).wrapping_sub(LSB);
//         // '?'
//         let is_qs = (block ^ QS).wrapping_sub(LSB);
//         // '#'
//         let is_hash = (block ^ HASH).wrapping_sub(LSB);
//
//         let result = (is_slash | is_qs | is_hash | block) & MSB;
//
//         if result != 0 {
//             let nth = (result.trailing_zeros() / 8) as usize;
//             return unsafe {
//                 state.as_ptr().offset_from_unsigned(bytes.as_ptr()) + nth
//             }
//         }
//
//         state = rest;
//     }
//
//     loop {
//         let [byte, rest @ ..] = state else {
//             return bytes.len();
//         };
//
//         if matches!(byte, b'/' | b'?' | b'#') || !byte.is_ascii() {
//             return unsafe { (byte as *const u8).offset_from_unsigned(bytes.as_ptr()) }
//         }
//
//         state = rest;
//     }
// }

