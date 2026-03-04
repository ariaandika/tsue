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
    // ===== 128 lookup table, usefull for ASCII byte =====
    {
        #[table_128]
        $(#[$meta:meta])*
        $vis:vis const unsafe fn $fn_id:ident($byte:ident:$u8:ty) { $e:expr }
    } => {
        $(#[$meta])*
        /// # Safety
        ///
        /// `byte` must be less than 128.
        $vis const unsafe fn $fn_id($byte: $u8) -> bool {
            const PAT: [bool; 128] = {
                let mut bytes = [false; 128];
                let mut $byte = 0u8;
                const fn filter($byte: $u8) -> bool {
                    $e
                }
                loop {
                    bytes[$byte as usize] = filter($byte);
                    if $byte == 127 {
                        break;
                    }
                    $byte += 1;
                }
                bytes
            };
            debug_assert!(byte < 128);
            // SAFETY: caller must ensure that `byte` is less than 128
            unsafe { *PAT.as_ptr().add($byte as usize) }
        }
    };
    // ===== 256 lookup table =====
    {
        $(#[$meta:meta])*
        $vis:vis const fn $fn_id:ident($byte:ident:$u8:ty) { $e:expr }
    } => {
        $(#[$meta])*
        $vis const fn $fn_id($byte: $u8) -> bool {
            const PAT: [bool; 256] = {
                let mut bytes = [false; 256];
                let mut $byte = 0u8;
                const fn filter($byte: $u8) -> bool {
                    $e
                }
                loop {
                    bytes[$byte as usize] = filter($byte);
                    if $byte == 255 {
                        break;
                    }
                    $byte += 1;
                }
                bytes
            };
            // SAFETY: the pattern size is equal to u8::MAX
            unsafe { *PAT.as_ptr().add($byte as usize) }
        }
    };
}

pub(crate) use {byte_map};

// ===== Blocks =====

byte_map! {
    /// unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"
    #[inline(always)]
    const fn unreserved(byte: u8) {
        byte.is_ascii_alphanumeric()
        || matches!(byte, b'-' | b'.' | b'_' | b'~')
    }
}

byte_map! {
    /// sub-delims = "!" / "$" / "&" / "'" / "(" / ")"
    ///            / "*" / "+" / "," / ";" / "="
    #[inline(always)]
    const fn sub_delims(byte: u8) {
        matches!(
            byte,
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
        )
        || byte.is_ascii_alphanumeric()
    }
}

byte_map! {
    /// pchar = unreserved / pct-encoded / sub-delims / ":" / "@"
    #[inline(always)]
    const fn is_pchar(byte: u8) {
        unreserved(byte)
        || matches!(byte, b'%')
        || sub_delims(byte)
        || matches!(byte, b':' | b'@')
    }
}

byte_map! {
    /// token   = 1*tchar
    /// tchar   = "!" / "#" / "$" / "%" / "&" / "'" / "*"
    ///         / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
    ///         / DIGIT / ALPHA
    #[inline(always)]
    pub const fn is_token(byte: u8) {
        matches!(
            byte,
            | b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*'
            | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
        )
        || byte.is_ascii_alphanumeric()
    }
}

// ===== lookup table =====

byte_map! {
    /// scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    #[inline(always)]
    pub const fn is_scheme(byte: u8) {
        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')
    }
}

byte_map! {
    /// userinfo = *( unreserved / pct-encoded / sub-delims / ":" )
    #[inline(always)]
    pub const fn is_userinfo(byte: u8) {
        unreserved(byte)
        || matches!(byte, b'%')
        || byte.is_ascii_hexdigit()
        || sub_delims(byte)
        || matches!(byte, b':')
    }
}

byte_map! {
    /// hex / ":" / "."
    ///
    /// this is temporary until ipv6 validation is implemented
    #[inline(always)]
    pub const fn is_ipv6(byte: u8) {
        byte.is_ascii_hexdigit() || matches!(byte, b':' | b'.')
    }
}

byte_map! {
    /// reg-name = *( unreserved / sub-delims / ":" )
    #[inline(always)]
    pub const fn is_ipvfuture(byte: u8) {
        unreserved(byte) || sub_delims(byte) || matches!(byte, b':')
    }
}

byte_map! {
    /// reg-name = *( unreserved / pct-encoded / sub-delims )
    #[inline(always)]
    pub const fn is_regname(byte: u8) {
        unreserved(byte) || matches!(byte, b'%') || byte.is_ascii_hexdigit() || sub_delims(byte)
    }
}

byte_map! {
    /// segment         = *pchar
    /// path-abempty    = *( "/" / segment )
    #[inline(always)]
    pub const fn is_path(byte: u8) {
        is_pchar(byte) || matches!(byte, b':' | b'@' | b'/')
    }
}

byte_map! {
    /// query = *( pchar / "/" / "?" )
    #[inline(always)]
    pub const fn is_query(byte: u8) {
        is_pchar(byte) || matches!(byte, b'/' | b'?')
    }
}

/// Any invalid character will have it MSB set.
///
/// Character is normalized to lowercase.
pub const HEADER_NAME: [u8; 256] = {
    let mut bytes = [0b10000000; 256];
    let mut i = 0u8;
    loop {
        if is_token(i) {
            bytes[i as usize] = i.to_ascii_lowercase();
        }
        if i == 127 {
            break;
        }
        i += 1;
    }
    bytes
};

// ===== SWAR =====

const BLOCK: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

pub const fn find_byte<const B: u8>(bytes: &[u8]) -> Option<usize> {
    const { assert!(B < 128) };

    let lf_ptr = 'swar: {
        let ch = usize::from_ne_bytes([B; BLOCK]);
        let mut state = bytes;

        while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
            let block = usize::from_ne_bytes(*chunk);
            let is_ch = (block ^ ch).wrapping_sub(LSB) & MSB;
            if is_ch != 0 {
                let nth = (is_ch.trailing_zeros() / 8) as usize;
                break 'swar unsafe { chunk.as_ptr().add(nth) };
            }
            state = rest;
        }

        while let [byte, rest @ ..] = state {
            if *byte == B {
                break 'swar byte as *const u8;
            } else {
                state = rest;
            }
        }

        return None;
    };

    let lf = unsafe { lf_ptr.offset_from_unsigned(bytes.as_ptr()) };
    // helps BytesMut::split* bounds checking
    unsafe { std::hint::assert_unchecked(lf < bytes.len()) };
    Some(lf)
}

// ===== hash =====

pub const PRIME_32: u32 = 0x0100_0193;
pub const BASIS_32: u32 = 0x811C_9DC5;

pub const fn hash_32(mut bytes: &[u8]) -> u32 {
    let mut hash = BASIS_32;

    while let [byte, rest @ ..] = bytes {
        hash = PRIME_32.wrapping_mul(hash ^ *byte as u32);
        bytes = rest;
    }

    hash
}

// pub const fn hash_64(mut bytes: &[u8]) -> u64 {
//     const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
//     const PRIME: u64 = 0x0100_0000_01b3;
//
//     let mut hash = BASIS;
//
//     while let [byte, rest @ ..] = bytes {
//         hash = PRIME.wrapping_mul(hash ^ *byte as u64);
//         bytes = rest;
//     }
//
//     hash
// }
