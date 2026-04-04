/// 128 lookup table
///
/// The input byte will be masked, caller must checks MSB themself.
macro_rules! ascii_lookup_table {
    {
        $(#[$meta:meta])*
        $vis:vis const fn $fn_id:ident($byte:ident:$u8:ty) -> bool {
            $e:expr
        }
    } => {
        $(#[$meta])*
        $vis const fn $fn_id($byte: $u8) -> bool {
            crate::matches::ascii_lookup_table! {
                const TABLE: [bool; 128] = fn($byte:$u8) -> bool { $e }
            }
            $byte.is_ascii() &
            // SAFETY: index masked by 127
            unsafe { *TABLE.as_ptr().add(($byte & 127) as usize) }
        }
    };
    {
        $(#[$meta:meta])*
        $vis:vis const $const_id:ident: [$t:ty; 128] = fn($byte:ident:$u8:ty) -> $t2:ty {
            $e:expr
        }
    } => {
        $(#[$meta])*
        $vis const $const_id: [$t; 128] = {
            let mut bytes = [false as $t; 128];
            let mut $byte = 0u8;
            const fn filter($byte: $u8) -> $t2 {
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
    }
}

pub(crate) use ascii_lookup_table;

// ===== Blocks =====

/// `pct-encoded = "%" HEXDIG HEXDIG`
pub const fn pct_encoded(byte: u8) -> bool {
    byte == b'%' || byte.is_ascii_hexdigit()
}

/// `unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"`
pub const fn unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// ```not_rust
/// sub-delims = "!" / "$" / "&" / "'" / "(" / ")"
///            / "*" / "+" / "," / ";" / "="
/// ```
pub const fn sub_delims(byte: u8) -> bool {
    matches!(
        byte,
        | b'!' | b'$' | b'&' | b'\'' | b'(' | b')'
        | b'*' | b'+' | b',' | b';' | b'='
    )
}

// ===== SWAR =====

const BLOCK: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

/// Returns the line without the delimiter.
pub const fn find_byte<const B: u8>(bytes: &[u8]) -> Option<&[u8]> {
    const { assert!(B < 128) };

    let ch = usize::from_ne_bytes([B; BLOCK]);
    let mut state = bytes;

    while let Some((chunk, rest)) = state.split_first_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);
        let is_ch = (block ^ ch).wrapping_sub(LSB) & MSB;
        if is_ch != 0 {
            unsafe {
                let nth = (is_ch.trailing_zeros() / 8) as usize;
                let end_ptr = state.as_ptr().add(nth);
                let len = end_ptr.offset_from_unsigned(bytes.as_ptr());
                return Some(std::slice::from_raw_parts(bytes.as_ptr(), len));
            }
        }
        state = rest;
    }

    loop {
        let [byte, rest @ ..] = state else {
            return None;
        };
        if *byte != B {
            unsafe {
                let end_ptr = state.as_ptr();
                let len = end_ptr.offset_from_unsigned(bytes.as_ptr());
                return Some(std::slice::from_raw_parts(bytes.as_ptr(), len));
            };
        }
        state = rest;
    }
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
