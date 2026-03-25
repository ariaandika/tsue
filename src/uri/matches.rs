use std::slice::from_raw_parts;

pub use crate::matches::*;

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

// ===== Util =====

/// Wrapping unchecked ascii to integer.
pub const fn atou(mut bytes: &[u8]) -> u16 {
    let mut o = 0;
    while let [lead, rest @ ..] = bytes {
        o *= 10;
        o += lead.wrapping_sub(b'0') as u16;
        bytes = rest;
    }
    o
}

// ===== SWAR =====

const BLOCK: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
const SLASH: usize = usize::from_ne_bytes([b'/'; BLOCK]);
const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
const AT: usize = usize::from_ne_bytes([b'@'; BLOCK]);

pub const fn find_path_delim(bytes: &[u8]) -> Option<usize> {
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

/// Split '@', delimiter is excluded.
pub const fn split_at_sign(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut state = bytes;

    while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);

        // '@'
        let is_at = (block ^ AT).wrapping_sub(LSB);

        let result = is_at & MSB;
        if result != 0 {
            unsafe {
                let nth = (result.trailing_zeros() / 8) as usize;
                let nth_ptr = state.as_ptr().add(nth);
                let end_ptr = bytes.as_ptr().add(bytes.len());

                let prefix_ptr = bytes.as_ptr();
                let prefix_len = nth_ptr.offset_from_unsigned(prefix_ptr);

                // skip the '@'
                let suffix_ptr = nth_ptr.add(1);
                let suffix_len = end_ptr.offset_from_unsigned(suffix_ptr);

                return Some((
                    from_raw_parts(prefix_ptr, prefix_len),
                    from_raw_parts(suffix_ptr, suffix_len),
                ));
            };
        }

        state = rest;
    }

    loop {
        let [byte, rest @ ..] = state else {
            return None;
        };
        if *byte == b'@' {
            unsafe {
                let prefix_ptr = bytes.as_ptr();
                let prefix_len = state.as_ptr().offset_from_unsigned(prefix_ptr);
                let prefix = from_raw_parts(prefix_ptr, prefix_len);
                return Some((prefix, rest));
            }
        }
        state = rest;
    }
}

#[test]
fn test_split_at_sign() {
    assert!(split_at_sign(b"example.com").is_none());

    let (left, right) = split_at_sign(b"user:passwd@example.com").unwrap();
    assert_eq!(left, b"user:passwd");
    assert_eq!(right, b"example.com");

    let (left, right) = split_at_sign(b"a@b").unwrap();
    assert_eq!(left, b"a");
    assert_eq!(right, b"b");

    let (left, right) = split_at_sign(b"user:passwd@b").unwrap();
    assert_eq!(left, b"user:passwd");
    assert_eq!(right, b"b");
}

/// Split ':'.
pub const fn split_port(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut state: &[u8] = bytes;

    loop {
        let [lead @ .., byte] = state else {
            return None;
        };
        if !byte.is_ascii_digit() {
            if *byte == b':' {
                unsafe {
                    let mid_ptr = bytes.as_ptr().add(state.len());
                    let len = bytes.len() - state.len();
                    return Some((lead, from_raw_parts(mid_ptr, len)));
                };
            } else {
                return None;
            }
        }
        state = lead;
    }
}

#[test]
fn test_split_port() {
    assert!(split_port(b"example.com").is_none());
    assert!(split_port(b"[a2f::1]").is_none());

    let (left, right) = split_port(b"example.com:443").unwrap();
    assert_eq!(left, b"example.com");
    assert_eq!(right, b"443");

    let (left, right) = split_port(b"[a2f::1]:443").unwrap();
    assert_eq!(left, b"[a2f::1]");
    assert_eq!(right, b"443");
}
