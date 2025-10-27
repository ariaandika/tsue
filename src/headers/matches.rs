pub(crate) use crate::matches::*;

byte_map! {
    /// Specialized `token` for lowercase only header name.
    ///
    /// token   = 1*tchar
    /// tchar   = "!" / "#" / "$" / "%" / "&" / "'" / "*"
    ///         / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
    ///         / DIGIT / ALPHA
    #[inline(always)]
    pub const fn is_token_lowercase(byte: u8) {
        matches!(
            byte,
            | b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*'
            | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            | b'0'..=b'9' | b'a'..=b'z'
        )
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

/// Returns (is_valid_header_name, is_valid_ascii).
///
/// field-value    = *field-content
/// field-content  = field-vchar
///                  [ 1*( SP / HTAB / field-vchar ) field-vchar ]
/// field-vchar    = VCHAR / obs-text
/// obs-text       = %x80-FF
#[inline(always)]
pub const fn is_header_value(byte: u8) -> (bool, bool) {
    const fn vchar(byte: u8) -> bool {
        matches!(byte, 0x21..=0x7E)
    }

    const fn obs_text(byte: u8) -> bool {
        matches!(byte, 0x80..=0xFF)
    }

    const fn valid(byte: u8) -> bool {
        vchar(byte) || obs_text(byte) || matches!(byte, b' ' | b'\t')
    }

    const OK: u8 = 0b01;
    const ASCII: u8 = 0b10;

    const PAT: [u8; 256] = {
        let mut bytes = [0b00; 256];
        let mut byte = 0u8;
        loop {
            if valid(byte) {
                // `(bool as u8) << 1` will mimic the ASCII flag
                bytes[byte as usize] = OK | (byte.is_ascii() as u8) << 1;
            }
            if byte == 255 {
                break;
            }
            byte += 1;
        }
        bytes
    };

    let flag = unsafe { *PAT.as_ptr().add(byte as usize) };
    (flag & OK == OK, flag & ASCII == ASCII)
}

pub const fn hash_32(mut bytes: &[u8]) -> u32 {
    const BASIS: u32 = 0x811C_9DC5;
    const PRIME: u32 = 0x0100_0193;

    let mut hash = BASIS;

    while let [byte, rest @ ..] = bytes {
        hash = PRIME.wrapping_mul(hash ^ *byte as u32);
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

