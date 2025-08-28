#[doc(hidden)]
macro_rules! byte_map {
    {
        const $cnid:ident =
            #[default($def:literal)]
            $(#[false]($nepat:pat))?
            $(#[true]($pat:pat))?
    } => {
        const $cnid: [bool; 256] = {
            let mut bytes = [$def; 256];
            let mut byte;
            $(
                byte = 0;
                loop {
                    if matches!(byte, $nepat) {
                        bytes[byte as usize] = false;
                    }
                    if byte == 255 {
                        break;
                    }
                    byte += 1;
                }
            )?
            $(
                byte = 0;
                loop {
                    if matches!(byte, $pat) {
                        bytes[byte as usize] = true;
                    }
                    if byte == 255 {
                        break;
                    }
                    byte += 1;
                }
            )?
            bytes
        };
    };
    ($byte:ident, $pat:pat) => {{
        const LUT: [bool; 256] = {
            let mut bytes = [false; 256];
            let mut byte = 0;
            loop {
                byte += 1;
                bytes[byte as usize] = matches!(byte, $pat);
                if byte == 255 {
                    break;
                }
            }
            bytes
        };
        LUT[$byte as usize]
    }};
    ($(#[$meta:meta])* pub const fn $fnid:ident($pat:pat)) => {
        $(#[$meta])*
        #[inline(always)]
        pub const fn $fnid(byte: u8) -> bool {
            const LUT: [bool; 256] = {
                let mut bytes = [false; 256];
                let mut byte = 0;
                loop {
                    byte += 1;
                    bytes[byte as usize] = matches!(byte, $pat);
                    if byte == 255 {
                        break;
                    }
                }
                bytes
            };
            LUT[byte as usize]
        }
    };
}

macro_rules! eq_block {
    ($block:ident, $byte:literal) => {
        (
            $block ^
            usize::from_ne_bytes([$byte; size_of::<usize>()])
        )
            .wrapping_sub(usize::from_ne_bytes([0b0000_0001; size_of::<usize>()]))
    };
}

// ===== General =====

/// `cursor.next()` returns ':', invalid character or `None`.
///
/// note that currently this does not comply with rfc, the following bytes will be passed:
///
/// - ",", ":", ";", "<", "=", ">", "\[", "\\", "]", "^", "_", "`"
macro_rules! match_scheme {
    ($cursor:ident else { $err:expr }) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const COLON: usize = usize::from_ne_bytes([b':'; BLOCK]);
            const SLASH: usize = usize::from_ne_bytes([b'/'; BLOCK]);
            const PLUS: usize = usize::from_ne_bytes([b'+'; BLOCK]);
            const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
            const AT: usize = usize::from_ne_bytes([b'@'; BLOCK]);
            const FIVE: usize = usize::from_ne_bytes([5; BLOCK]);

            while let Some(chunk) = $cursor.peek_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // most checks does not handle `byte >= 128`,
                // because its already checked with `.. | block) & ..`

                // ":"
                let is_cl = (block ^ COLON).wrapping_sub(LSB);
                // "/"
                let is_sl = (block ^ SLASH).wrapping_sub(LSB);
                // "?"
                let is_qs = (block ^ QS).wrapping_sub(LSB);
                // "@"
                let is_at = (block ^ AT).wrapping_sub(LSB);
                // 43(b'+') < byte
                let lt_pl = block.wrapping_sub(PLUS);
                // 122(b'z') > byte
                let gt_z = block.saturating_add(FIVE);

                let result = (is_cl | is_sl | is_qs | is_at | lt_pl | gt_z | block) & MSB;
                if result != 0 {
                    $cursor.advance((result.trailing_zeros() / 8) as usize);
                    break 'swar;
                }

                $cursor.advance(BLOCK);
            }

            while let Some(byte) = $cursor.next() {
                simd::byte_map! {
                    const PAT =
                        #[default(true)]
                        #[false](b'+'..=b'z')
                        #[true](b':' | b'/' | b'?' | b'@')
                }

                if PAT[byte as usize] {
                    $cursor.step_back(1);
                    break 'swar;
                }
            }

            $err
        }
    }
}

/// `cursor.next()` returns '/', '?', '#', invalid character or `None`.
macro_rules! match_authority {
    ($cursor:ident) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
            const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
            const SLASH: usize = usize::from_ne_bytes([b'/'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            while let Some(chunk) = $cursor.peek_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // '/'
                let is_sl = (block ^ SLASH).wrapping_sub(LSB);
                // '?'
                let is_qs = (block ^ QS).wrapping_sub(LSB);
                // '#'
                let is_hs = (block ^ HASH).wrapping_sub(LSB);
                // 33('!') < byte
                let lt_33 = block.wrapping_sub(BANG);
                // 127(DEL)
                let is_del = (block ^ DEL).wrapping_sub(LSB);

                let result = (is_sl | is_qs | is_hs | lt_33 | is_del | block) & MSB;
                if result != 0 {
                    $cursor.advance((result.trailing_zeros() / 8) as usize);
                    break 'swar;
                }

                $cursor.advance(BLOCK);
            }

            while let Some(byte) = $cursor.next() {
                simd::byte_map! {
                    const PAT =
                        #[default(true)]
                        #[false](b'!'..=b'~')
                        #[true](b'/' | b'?' | b'#')
                }

                if PAT[byte as usize] {
                    $cursor.step_back(1);
                    break 'swar;
                }
            }
        }
    };
}

/// `cursor.next()` returns '?', '#', invalid character or `None`.
///
/// Postcondition: advanced bytes is valid ASCII.
macro_rules! match_path {
    ($cursor:ident) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
            const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            while let Some(chunk) = $cursor.peek_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // '?'
                let is_qs = (block ^ QS).wrapping_sub(LSB);
                // '#'
                let is_hs = (block ^ HASH).wrapping_sub(LSB);
                // 33('!') < byte
                let lt_33 = block.wrapping_sub(BANG);
                // 127(DEL)
                let is_del = (block ^ DEL).wrapping_sub(LSB);

                let result = (is_qs | is_hs | lt_33 | is_del | block) & MSB;
                if result != 0 {
                    $cursor.advance((result.trailing_zeros() / 8) as usize);
                    break 'swar;
                }

                $cursor.advance(BLOCK);
            }

            while let Some(byte) = $cursor.next() {
                simd::byte_map! {
                    const PAT =
                        #[default(true)]
                        // byte matching this will not trigger `break`
                        #[false](b'!'..=b'~')
                        // exclusively this pattern
                        #[true](b'?' | b'#')
                }

                if PAT[byte as usize] {
                    $cursor.step_back(1);
                    break 'swar;
                }
            }

            // contains full path
        }
    };
}

/// `cursor.next()` returns '#', invalid character or `None`.
///
/// Postcondition: advanced bytes is valid ASCII.
macro_rules! match_query {
    ($cursor:ident) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            while let Some(chunk) = $cursor.peek_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // '#'
                let is_hs = (block ^ HASH).wrapping_sub(LSB);
                // 33('!') < byte
                let lt_33 = block.wrapping_sub(BANG);
                // 127(DEL)
                let is_del = (block ^ DEL).wrapping_sub(LSB);

                let result = (is_hs | lt_33 | is_del | block) & MSB;
                if result != 0 {
                    $cursor.advance((result.trailing_zeros() / 8) as usize);
                    break 'swar;
                }

                $cursor.advance(BLOCK);
            }

            while let Some(byte) = $cursor.next() {
                simd::byte_map! {
                    const PAT =
                        #[default(true)]
                        // byte matching this will not trigger `break`
                        #[false](b'!'..=b'~')
                        // exclusively this pattern
                        #[true](b'#')
                }

                if PAT[byte as usize] {
                    $cursor.step_back(1);
                    break 'swar;
                }
            }

            // contains no fragment
        }
    };
}

/// `cursor.next()` returns invalid character or None.
///
/// invalid character is not any of: b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'-' | b'.'
///
/// no simd, scheme is generally short, and too complicated for simd logic.
macro_rules! validate_scheme {
    ($value:ident else { $err:expr }) => {
        let mut cursor = $value.cursor();

        while let Some(byte) = cursor.next() {
            if !simd::byte_map!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'-' | b'.') {
                cursor.step_back(1);
                $err
            }
        }
    };
}

/// `cursor.next()` returns '@', invalid character or None.
///
/// invalid character is: b'#' | b'/' | b'<' | b'>' | b'?' | b'\[' | b'\\' | b']' | b'^' |
/// b'`' | b'{' | b'|' | b'}'
macro_rules! validate_authority {
    ($cursor:ident) => {{
        let mut col = None;

        while let Some(byte) = $cursor.next() {
            simd::byte_map! {
                const PAT =
                    #[default(false)]
                    #[true](
                        b'#' | b'/' | b'<' | b'>' | b'?' | b'[' | b'\\' |
                        b']' | b'^' | b'`' | b'{' | b'|' | b'}' | b'@'
                    )
            }

            if PAT[byte as usize] {
                $cursor.step_back(1);
                break;
            } else if byte == b':' {
                col = Some($cursor.steps())
            }
        }

        col
    }};
}

macro_rules! match_port_rev {
    ($cursor:ident else { $expr:expr }) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const COL: usize = usize::from_ne_bytes([b':'; BLOCK]);

            while let Some(chunk) = $cursor.peek_prev_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // ':'
                let result = (block ^ COL).wrapping_sub(LSB) & MSB;

                if result != 0 {
                    $cursor.step_back(8 - (result.trailing_zeros() / 8) as usize);
                    break 'swar;
                }

                $cursor.step_back(BLOCK);
            }

            while let Some(byte) = $cursor.prev() {
                if byte == b':' {
                    break 'swar;
                }
            }

            $expr
        }
    };
}

byte_map! {
    pub const fn is_digit(b'0'..=b'9')
}

pub(crate) use {
    byte_map, eq_block, /* inverted_byte_map, */ match_query, match_path, match_scheme,
    validate_authority, validate_scheme, match_port_rev, match_authority
};

