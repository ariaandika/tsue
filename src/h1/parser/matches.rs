pub(crate) use crate::matches::*;

/// Find and split slice for a crlf (`\r\n`) and check for ASCII.
///
/// # Usage
///
/// ```not_rust
/// let mut state: &[u8];
///
/// let (delimiter, leading) = matches::split_crlf!(state else {
///     panic!("crlf not found")
/// });
///
/// assert!(matches!(delimiter, b'\r' | b'\n') || !delimiter.is_ascii());
///
/// // `state` contains the rest of the bytes
/// ```
macro_rules! split_crlf {
    ($state:ident else { $none:expr }) => {
        'swar: {
            use std::slice::from_raw_parts;
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const CR: usize = usize::from_ne_bytes([b'\r'; BLOCK]);
            const LF: usize = usize::from_ne_bytes([b'\n'; BLOCK]);

            while let Some((chunk, rest)) = $state.split_first_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // '\r'
                let is_cr = (block ^ CR).wrapping_sub(LSB);
                // '\n'
                let is_lf = (block ^ LF).wrapping_sub(LSB);

                let result = (is_cr | is_lf | block) & MSB;
                if result == 0 {
                    $state = rest;
                    continue;
                }

                let nth = (result.trailing_zeros() / 8) as usize;
                unsafe {
                    let end = $state.as_ptr().add($state.len());
                    let at_ptr = $state.as_ptr().add(nth);

                    let rest = at_ptr.add(1);
                    let rest_len = end.offset_from_unsigned(rest);
                    $state = from_raw_parts(rest, rest_len);

                    break 'swar *at_ptr
                };
            }

            while let [byte, rest @ ..] = $state {
                if matches!(byte, b'\r' | b'\n') || !byte.is_ascii() {
                    $state = rest;
                    break 'swar *byte
                }

                $state = rest;
            }

            $none
        }
    };
}

pub(crate) use {split_crlf};

macro_rules! split_header_name {
    ($state:ident else { $none:expr }) => {
        'swar: {
            use std::slice::from_raw_parts;
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const COL: usize = usize::from_ne_bytes([b':'; BLOCK]);
            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            while let Some((chunk, rest)) = $state.split_first_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // ':'
                let is_col = (block ^ COL).wrapping_sub(LSB);
                // <= '!'
                let lt_33 = block.wrapping_sub(BANG);
                // 127(DEL)
                let is_del = (block ^ DEL).wrapping_sub(LSB);

                let result = (is_col | lt_33 | is_del | block) & MSB;
                if result == 0 {
                    $state = rest;
                    continue;
                }

                let nth = (result.trailing_zeros() / 8) as usize;
                unsafe {
                    let end = $state.as_ptr().add($state.len());
                    let at_ptr = $state.as_ptr().add(nth);

                    let rest = at_ptr.add(1);
                    let rest_len = end.offset_from_unsigned(rest);
                    $state = from_raw_parts(rest, rest_len);

                    break 'swar *at_ptr
                };
            }

            while let [byte, rest @ ..] = $state {
                if matches!(byte, b':') || !byte.is_ascii() {
                    $state = rest;
                    break 'swar *byte
                }

                $state = rest;
            }

            $none
        }
    };
}

pub(crate) use {split_header_name};
