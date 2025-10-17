pub(crate) use crate::matches::*;

byte_map! {
    /// field-name = token
    #[inline(always)]
    pub const fn is_field_name_char(byte: u8) {
        self::is_token(byte)
    }
}

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
