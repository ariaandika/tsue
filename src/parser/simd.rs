
/// Pointer size.
pub const BLOCK: usize = size_of::<usize>();
/// Block of most significant bit.
pub const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
/// Block of least significant bit.
pub const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

/// Block of ":".
pub const COLON: usize = usize::from_ne_bytes([b':'; BLOCK]);
/// Block of "#".
pub const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
/// Block of "?".
pub const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);

/// Block of DEL byte, used for validating ASCII.
pub const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

/// Generic SWAR find 1 char implementation.
///
/// ```not_rust
/// // `cursor.next()` will still have '\n'
/// let poll: Poll<()> = match_simd!(cursor, b'\n')
///
/// // `cursor.next()` will NOT have '\n'
/// let poll: Poll<()> = match_simd!(cursor, =b'\n')
/// ```
macro_rules! match_swar {
    (@build{$($add:tt)*} $cursor:expr, $b:literal) => {
        'swar: {
            use crate::parser::simd::{BLOCK, MSB, LSB};
            const DATA: usize = usize::from_ne_bytes([$b; BLOCK]);

            while let Some(chunk) = $cursor.peek_chunk::<BLOCK>() {
                let value = usize::from_ne_bytes(*chunk);
                let lf_xor = value ^ DATA;
                let lf_result = lf_xor.wrapping_sub(LSB) & !lf_xor & MSB;

                if lf_result != 0 {
                    let lf_pos = (lf_result.trailing_zeros() / 8) as usize;
                    $cursor.advance(lf_pos $($add)*);
                    break 'swar std::task::Poll::Ready(());
                }

                $cursor.advance(crate::parser::simd::BLOCK);
            }

            while let Some(b) = $cursor.next() {
                if b == $b {
                    break 'swar std::task::Poll::Ready(());
                }
            }

            std::task::Poll::Pending
        }
    };

    ($cursor:expr, =$b:literal) => {
        match_swar!(@build{+ 1} $cursor, $b)
    };
    ($cursor:expr, $b:literal) => {
        match_swar!(@build{} $cursor, $b)
    };
}

macro_rules! not_ascii_block {
    ($value:ident) => {
        // validate ASCII (< 127)
        ($value.wrapping_sub(crate::parser::simd::DEL) & !$value & crate::parser::simd::MSB)
            != crate::parser::simd::MSB
    };
}

pub(crate) use {match_swar, not_ascii_block};
