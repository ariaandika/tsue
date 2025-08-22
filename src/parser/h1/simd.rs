
macro_rules! match_crlf {
    ($cursor:ident) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const CR: usize = usize::from_ne_bytes([b'\r'; BLOCK]);
            const LF: usize = usize::from_ne_bytes([b'\n'; BLOCK]);

            while let Some(chunk) = $cursor.peek_chunk() {
                let block = usize::from_ne_bytes(*chunk);

                // this does not handle (byte >= 128),
                // but it checked below
                // '\r'
                let is_cr = (block ^ CR).wrapping_sub(LSB);

                // '\n'
                let is_lf = (block ^ LF).wrapping_sub(LSB);

                let result = (is_cr | is_lf | block) & MSB;
                if result != 0 {
                    $cursor.advance((result.trailing_zeros() / 8) as usize);
                    break 'swar;
                }

                $cursor.advance(BLOCK);
            }

            while let Some(byte) = $cursor.next() {
                if matches!(byte, b'\r' | b'\n') || byte >= 128 {
                    $cursor.step_back(1);
                    break 'swar;
                }
            }

            return Poll::Pending;
        }
    };
}

pub(crate) use {match_crlf};
