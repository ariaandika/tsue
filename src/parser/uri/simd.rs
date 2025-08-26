#[doc(hidden)]
macro_rules! inverted_byte_map {
    (const $cnid:ident = #[false]($nepat:pat) #[true]($pat:pat)) => {
        const $cnid: [bool; 256] = {
            let mut bytes = [true; 256];
            let mut byte = 0;
            loop {
                byte += 1;
                bytes[byte as usize] = !matches!(byte, $nepat);
                if byte == 255 {
                    break;
                }
            }
            byte = 0;
            loop {
                byte += 1;
                bytes[byte as usize] = matches!(byte, $pat);
                if byte == 255 {
                    break;
                }
            }
            bytes
        };
    };
}
#[doc(hidden)]
macro_rules! byte_map {
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

macro_rules! match_uri_leader {
    ($cursor:ident else { $err:expr }) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const COLON: usize = usize::from_ne_bytes([b':'; BLOCK]);
            const SLASH: usize = usize::from_ne_bytes([b'/'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            while let Some(chunk) = $cursor.peek_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // most checks does not handle `byte >= 128`,
                // because its already checked with `.. | block) & ..`

                // ":"
                let is_cl = (block ^ COLON).wrapping_sub(LSB);
                // "/"
                let is_sl = (block ^ SLASH).wrapping_sub(LSB);
                // 33(b'!') <= byte
                let lt_33 = block.wrapping_sub(BANG);
                // 127(DEL)
                let is_del = (block ^ DEL).wrapping_sub(LSB);

                let result = (is_cl | is_sl | lt_33 | is_del | block) & MSB;
                if result != 0 {
                    $cursor.advance((result.trailing_zeros() / 8) as usize);
                    break 'swar;
                }

                $cursor.advance(BLOCK);
            }

            while let Some(byte) = $cursor.next() {
                if matches!(byte, b':' | b'/') || !matches!(byte, b'!'..=b'~') {
                    $cursor.step_back(1);
                    break 'swar;
                }
            }

            $err
        }
    }
}

/// `cursor.next()` returns '?', '#', invalid character or `None`.
///
/// invalid character is not any of: `b'!'..=b'~'`.
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
                simd::inverted_byte_map! {
                    const PAT =
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
/// invalid character is not any of: `b'!'..=b'~'`.
macro_rules! match_fragment {
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
                simd::inverted_byte_map! {
                    const PAT =
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

pub(crate) use {
    byte_map, eq_block, match_path, match_uri_leader, match_fragment, validate_scheme, inverted_byte_map,
};

