macro_rules! byte_map {
    {
        $(#[$meta:meta])*
        $vis:vis const $cnid:ident =
            #[default($def:literal)]
            $(#[false]($nepat:pat))?
            $(#[true]($pat:pat))?
    } => {
        $(#[$meta])*
        $vis const $cnid: [bool; 256] = {
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
    // =====
    {
        $(#[$meta:meta])*
        $vis:vis const fn $fn_id:ident(
            default: $def:literal,
            $(false: $nepat:pat,)?
            $(true: $pat:pat,)?
        );
    } => {
        $(#[$meta])*
        $vis const fn $fn_id(byte: u8) -> bool {
            static PAT: [bool; 256] = {
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
            const PTR: *const bool = PAT.as_ptr();
            // SAFETY: the pattern size is equal to u8::MAX
            unsafe { *PTR.add(byte as usize) }
        }
    };
}

// ===== lookup table =====

byte_map! {
    #[inline(always)]
    pub const fn is_hex(
        default: false,
        true: b'a'..=b'f' | b'A'..=b'F' | b'0'..=b'9',
    );
}

byte_map! {
    /// scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    #[inline(always)]
    pub const fn is_scheme(
        default: false,
        true:
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' |
            b'+' | b'-' | b'.',
    );
}

byte_map! {
    /// userinfo = *( unreserved / pct-encoded / sub-delims / ":" )
    #[inline(always)]
    pub const fn is_userinfo(
        default: false,
        true:
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' |
            b'%' |
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' |
            b':',
    );
}

byte_map! {
    /// hex / ":" / "."
    #[inline(always)]
    pub const fn is_ipv6(
        default: false,
        true:
            b'a'..=b'f' | b'A'..=b'F' | b'0'..=b'9' |
            b':' |
            b'.',
    );
}

byte_map! {
    /// reg-name = *( unreserved / sub-delims / ":" )
    #[inline(always)]
    pub const fn is_ipvfuture(
        default: false,
        true:
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' |
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' |
            b':',
    );
}

byte_map! {
    /// reg-name = *( unreserved / pct-encoded / sub-delims )
    #[inline(always)]
    pub const fn is_regname(
        default: false,
        true:
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' |
            b'%' |
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=',
    );
}

macro_rules! match_query {
    (
        $bytes:expr;
        |$val:ident,$cursor:ident|$matches:expr;
        else { $el:expr }
    ) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
            const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            let mut $cursor = $bytes.cursor_mut();

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
                    let nth = (result.trailing_zeros() / 8) as usize;
                    // SAFETY: `cursor.peek_chunk::<BLOCK>()` returns `Some`,
                    // `nth` is less than `BLOCK`
                    let $val = unsafe {
                        $cursor.advance_unchecked(nth);
                        chunk.get_unchecked(nth)
                    };
                    break 'swar $matches;
                }

                $cursor.advance(BLOCK);
            }

            while let Some($val) = $cursor.next() {
                matches::byte_map! {
                    const PAT =
                        #[default(true)]
                        #[false](b'!'..=b'~')
                        #[true](b'?' | b'#')
                }

                if PAT[$val as usize] {
                    $cursor.step_back(1);
                    break 'swar $matches;
                }
            }

            $el
        }
    };
}

macro_rules! match_fragment {
    (
        $cursor:expr;
        |$val:ident|$matches:expr;
    ) => {
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
                    let nth = (result.trailing_zeros() / 8) as usize;
                    $cursor.advance(nth);
                    let $val = chunk[nth];
                    break 'swar $matches;
                }

                $cursor.advance(BLOCK);
            }

            while let Some($val) = $cursor.next() {
                matches::byte_map! {
                    const PAT =
                        #[default(true)]
                        #[false](b'!'..=b'~')
                        #[true](b'#')
                }

                if PAT[$val as usize] {
                    $cursor.step_back(1);
                    break 'swar $matches;
                }
            }
        }
    };
}

pub(crate) use {byte_map, match_fragment, match_query};

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

macro_rules! split_col {
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
            const COL: usize = usize::from_ne_bytes([b':'; BLOCK]);

            let original = $bytes;
            let mut state: &[u8] = original;

            while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
                let $block = usize::from_ne_bytes(*chunk);

                // ':'
                let is_col = ($block ^ COL).wrapping_sub(LSB);

                let result = (is_col $($ascii)*) & MSB;
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
                if *$block == b':' $($ascii_iter)* {
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
        matches::split_col! {
            #[private]
            #[block = block]
            #[ascii = ]
            #[ascii_iter = ]
            $bytes
        }
    };
    ($bytes:expr) => {
        matches::split_col! {
            #[private]
            #[block = block]
            #[ascii = | block]
            #[ascii_iter = || !block.is_ascii()]
            $bytes
        }
    };
}

pub(crate) use {split_col};

