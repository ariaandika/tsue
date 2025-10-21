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
        if i == 255 {
            break;
        }
        i += 1;
    }
    bytes
};

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

