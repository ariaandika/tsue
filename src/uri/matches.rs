pub(crate) use crate::matches::*;

macro_rules! split_at_sign {
    (
        #[private]
        #[block = $block:ident]
        #[ascii = $($ascii:tt)*]
        #[ascii_iter = $($ascii_iter:tt)*]
        $bytes:expr
    ) => {
        'swar: {
            use std::slice::from_raw_parts;
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const AT: usize = usize::from_ne_bytes([b'@'; BLOCK]);

            let original = $bytes;
            let mut state: &[u8] = original;

            while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
                let $block = usize::from_ne_bytes(*chunk);

                // '@'
                let is_at = ($block ^ AT).wrapping_sub(LSB);

                let result = (is_at $($ascii)*) & MSB;
                if result != 0 {
                    let nth = (result.trailing_zeros() / 8) as usize;
                    break 'swar unsafe {
                        let start = original.as_ptr();
                        let mid_ptr = chunk.as_ptr().add(nth);
                        let mid = mid_ptr.offset_from_unsigned(original.as_ptr());
                        Some((
                            from_raw_parts(start, mid),
                            from_raw_parts(mid_ptr, original.len().unchecked_sub(mid)),
                        ))
                    };
                }

                state = rest;
            }

            while let [$block, rest @ ..] = state {
                if *$block == b'@' $($ascii_iter)* {
                    break 'swar unsafe {
                        let start = original.as_ptr();
                        let mid_ptr = state.as_ptr();
                        let mid = mid_ptr.offset_from_unsigned(original.as_ptr());
                        Some((
                            from_raw_parts(start, mid),
                            from_raw_parts(mid_ptr, original.len().unchecked_sub(mid)),
                        ))
                    };
                } else {
                    state = rest;
                }
            }

            None
        }
    };

    // user input
    (#[skip_ascii]$bytes:expr) => {
        matches::split_at_sign! {
            #[private]
            #[block = block]
            #[ascii = ]
            #[ascii_iter = ]
            $bytes
        }
    };
    ($bytes:expr) => {
        matches::split_at_sign! {
            #[private]
            #[block = block]
            #[ascii = | block]
            #[ascii_iter = || !block.is_ascii()]
            $bytes
        }
    };
}

pub(crate) use {split_at_sign};

/// Find colon from the end.
///
/// Backwards finding is necessary to avoid conflict with ipv6.
macro_rules! split_port {
    ($bytes:expr) => {
        'swar: {
            use std::slice::from_raw_parts;
            let original = $bytes;
            let mut state: &[u8] = original;

            while let [rest @ .., byte] = state {
                if byte.is_ascii_digit() {
                    state = rest;
                } else if *byte == b':' {
                    break 'swar unsafe {
                        let mid_ptr = state.as_ptr().add(state.len());
                        let len = original.len() - state.len();
                        Some((
                            rest,
                            from_raw_parts(mid_ptr, len),
                        ))
                    };
                } else {
                    break 'swar None
                }
            }

            None
        }
    };
}

pub(crate) use {split_port};

macro_rules! find_path_delim {
    ($bytes:expr) => {
        'swar: {
            use std::slice::from_raw_parts;
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
                        let start = original.as_ptr();
                        let len = state.as_ptr().offset_from_unsigned(start);
                        Some(from_raw_parts(start, len + nth))
                    }
                }

                state = rest;
            }

            while let [byte, rest @ ..] = state {
                if matches!(byte, b'/' | b'?' | b'#') || !byte.is_ascii() {
                    break 'swar unsafe {
                        let start = original.as_ptr();
                        let len = rest.as_ptr().offset_from_unsigned(start);
                        Some(from_raw_parts(start, len))
                        // Some(from_raw_parts(original.as_ptr(), rest.as_ptr().offset_from_unsigned(original.as_ptr())))
                    }
                } else {
                    state = rest;
                }
            }

            None
        }
    };
}

pub(crate) use {find_path_delim};

