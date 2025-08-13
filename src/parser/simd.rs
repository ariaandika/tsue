/// Generic SWAR find 1 char implementation.
///
/// ```not_rust
/// // `cursor.next()` will still have '\n'
/// let poll: Poll<()> = match_simd!(cursor, b'\n')
///
/// // `cursor.next()` will NOT have '\n'
/// let poll: Poll<()> = match_simd!(cursor, =b'\n')
/// ```
macro_rules! match_simd {
    (@build{$($add:tt)*} $cursor:expr, $b:literal) => {
        'simd: {
            const CHUNK_SIZE: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([128; CHUNK_SIZE]);
            const LSB: usize = usize::from_ne_bytes([1; CHUNK_SIZE]);
            const DATA: usize = usize::from_ne_bytes([$b; CHUNK_SIZE]);
            while let Some(chunk) = $cursor.peek_chunk::<CHUNK_SIZE>() {
                let value = usize::from_ne_bytes(*chunk);
                let lf_xor = value ^ DATA;
                let lf_result = lf_xor.wrapping_sub(LSB) & !lf_xor & MSB;

                if lf_result != 0 {
                    let lf_pos = (lf_result.trailing_zeros() / 8) as usize;
                    $cursor.advance(lf_pos $($add)*);
                    break 'simd std::task::Poll::Ready(());
                }

                $cursor.advance(CHUNK_SIZE);
            }
            while let Some(b) = $cursor.next() {
                if b == b'\n' {
                    break 'simd std::task::Poll::Ready(());
                }
            }
            std::task::Poll::Pending
        }
    };

    ($cursor:expr, =$b:literal) => {
        match_simd!(@build{+ 1} $cursor, $b)
    };
    ($cursor:expr, $b:literal) => {
        match_simd!(@build{()} $cursor, $b)
    };
}
pub(crate) use {match_simd};
