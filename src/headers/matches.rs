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

pub const fn hash(bytes: &[u8]) -> u64 {
    const INITIAL_STATE: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = INITIAL_STATE;
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i].to_ascii_lowercase() as u64;
        hash = hash.wrapping_mul(PRIME);
        i += 1;
    }

    hash
}

pub use hash as hash_to_lowercase;
