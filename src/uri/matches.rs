use std::slice::from_raw_parts;

pub(crate) use crate::matches::*;

macro_rules! split_delim_chunk {
    ($bytes:ident,$chunk:ident,$nth:ident) => {
        unsafe {
            let nth_ptr = $chunk.as_ptr().add($nth);
            let end_ptr = $bytes.as_ptr().add($bytes.len());

            let start = $bytes.as_ptr();
            let start_len = nth_ptr.offset_from_unsigned(start);

            let end = nth_ptr.add(1);
            let end_len = end_ptr.offset_from_unsigned(end);
            Some((
                from_raw_parts(start, start_len),
                from_raw_parts(end, end_len),
            ))
        }
    };
}

macro_rules! split_delim {
    ($bytes:ident,$state:ident,$rest:ident) => {{
        let start = $bytes.as_ptr();
        let lead = unsafe { from_raw_parts(start, $state.as_ptr().offset_from_unsigned(start)) };
        Some((lead, $rest))
    }};
}

/// Split '@'.
pub const fn split_at_sign(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    const BLOCK: usize = size_of::<usize>();
    const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
    const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
    const AT: usize = usize::from_ne_bytes([b'@'; BLOCK]);

    let mut state: &[u8] = bytes;

    while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);

        // '@'
        let is_at = (block ^ AT).wrapping_sub(LSB);

        let result = is_at & MSB;
        if result != 0 {
            let nth = (result.trailing_zeros() / 8) as usize;
            return split_delim_chunk!(bytes, chunk, nth);
        }

        state = rest;
    }

    while let [byte, rest @ ..] = state {
        if *byte == b'@' {
            return split_delim!(bytes, state, rest);
        }

        state = rest;
    }

    None
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

    while let [lead @ .., byte] = state {
        if byte.is_ascii_digit() {
            state = lead;
        } else if *byte == b':' {
            unsafe {
                let mid_ptr = state.as_ptr().add(state.len());
                let len = bytes.len() - state.len();
                return Some((lead, from_raw_parts(mid_ptr, len)));
            };
        } else {
            return None;
        }
    }

    None
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

macro_rules! find_path_delim {
    ($bytes:expr) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const SLASH: usize = usize::from_ne_bytes([b'/'; BLOCK]);
            const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
            const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);

            let original = $bytes;
            let mut state: &[u8] = original;

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
                    break 'swar unsafe {
                        Some(state.as_ptr().offset_from_unsigned(original.as_ptr()) + nth)
                    }
                }

                state = rest;
            }

            while let [byte, rest @ ..] = state {
                if matches!(byte, b'/' | b'?' | b'#') || !byte.is_ascii() {
                    break 'swar unsafe {
                        Some(state.as_ptr().offset_from_unsigned(original.as_ptr()))
                    }
                }

                state = rest;
            }

            None
        }
    };
}

pub(crate) use {find_path_delim};

