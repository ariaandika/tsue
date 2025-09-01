macro_rules! byte_map {
    {
        const $cnid:ident =
            #[default($def:literal)]
            $(#[false]($nepat:pat))?
            $(#[true]($pat:pat))?
    } => {
        const $cnid: [bool; 256] = {
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
}

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

                // most checks does not handle `byte >= 128`,
                // because its already checked with `.. | block) & ..`

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
        }
    };
}

macro_rules! match_target {
    ($cursor:expr; |$arg:ident|$matches:expr; else { $el:expr }) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);
            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            while let Some(chunk) = $cursor.peek_chunk() {
                let block = usize::from_ne_bytes(*chunk);

                // <= '!'
                let lt_33 = block.wrapping_sub(BANG);
                // 127(DEL)
                let is_del = (block ^ DEL).wrapping_sub(LSB);

                let result = (lt_33 | is_del | block) & MSB;
                if result != 0 {
                    let nth = (result.trailing_zeros() / 8) as usize;
                    $cursor.advance(nth + 1);
                    let $arg = chunk[nth];
                    break 'swar $matches;
                }

                $cursor.advance(BLOCK);
            }

            while let Some($arg) = $cursor.next() {
                simd::byte_map! {
                    const PAT =
                        #[default(true)]
                        #[false](b'!'..=b'~')
                }

                dbg!($arg);
                if PAT[$arg as usize] {
                    break 'swar $matches;
                }
            }

            $el
        }
    };
}

pub(crate) use {match_crlf, match_target, byte_map};
