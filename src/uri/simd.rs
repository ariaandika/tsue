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

macro_rules! validate_scheme {
    (
        $bytes:expr;
        else { $err:expr }
    ) => {
        {
            let mut cursor = $bytes.cursor();
            while let Some(byte) = cursor.next() {
                simd::byte_map! {
                    const PAT =
                        #[default(true)]
                        #[false](
                            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' |
                            b'+' | b'-' | b'.'
                        )
                }

                if PAT[byte as usize] {
                    $err
                }
            }
        }
    }
}

macro_rules! validate_authority {
    (
        $bytes:expr;
        else { $err:expr }
    ) => {
        {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
            const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
            const SLASH: usize = usize::from_ne_bytes([b'/'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            let mut cursor = $bytes.cursor();

            while let Some(chunk) = cursor.peek_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // '/'
                let is_sl = (block ^ SLASH).wrapping_sub(LSB);
                // '?'
                let is_qs = (block ^ QS).wrapping_sub(LSB);
                // '#'
                let is_hs = (block ^ HASH).wrapping_sub(LSB);
                // 33('!') < byte
                let lt_33 = block.wrapping_sub(BANG);
                // 127(DEL)
                let is_del = (block ^ DEL).wrapping_sub(LSB);

                let result = (is_sl | is_qs | is_hs | lt_33 | is_del | block) & MSB;
                if result != 0 {
                    $err
                }

                cursor.advance(BLOCK);
            }

            while let Some(byte) = cursor.next() {
                simd::byte_map! {
                    const PAT =
                        #[default(true)]
                        #[false](b'!'..=b'~')
                        #[true](b'/' | b'?' | b'#')
                }

                if PAT[byte as usize] {
                    $err
                }
            }
        }
    };
}

macro_rules! match_query {
    (
        $bytes:expr;
        |$val:ident,$cursor:ident|$matches:expr;
        else { $el:expr }
    ) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
            const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            let mut $cursor = $bytes.cursor_mut();

            while let Some(chunk) = $cursor.peek_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // '?'
                let is_qs = (block ^ QS).wrapping_sub(LSB);
                // '#'
                let is_hs = (block ^ HASH).wrapping_sub(LSB);
                // 33('!') < byte
                let lt_33 = block.wrapping_sub(BANG);
                // 127(DEL)
                let is_del = (block ^ DEL).wrapping_sub(LSB);

                let result = (is_qs | is_hs | lt_33 | is_del | block) & MSB;
                if result != 0 {
                    let nth = (result.trailing_zeros() / 8) as usize;
                    $cursor.advance(nth);
                    let $val = chunk[nth];
                    break 'swar $matches;
                }

                $cursor.advance(BLOCK);
            }

            while let Some($val) = $cursor.next() {
                simd::byte_map! {
                    const PAT =
                        #[default(true)]
                        #[false](b'!'..=b'~')
                        #[true](b'?' | b'#')
                }

                if PAT[$val as usize] {
                    $cursor.step_back(1);
                    break 'swar $matches;
                }
            }

            $el
        }
    };
}

macro_rules! match_fragment {
    (
        $cursor:expr;
        |$val:ident|$matches:expr;
    ) => {
        'swar: {
            const BLOCK: usize = size_of::<usize>();
            const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
            const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

            const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
            const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
            const DEL: usize = usize::from_ne_bytes([127; BLOCK]);

            while let Some(chunk) = $cursor.peek_chunk::<BLOCK>() {
                let block = usize::from_ne_bytes(*chunk);

                // '#'
                let is_hs = (block ^ HASH).wrapping_sub(LSB);
                // 33('!') < byte
                let lt_33 = block.wrapping_sub(BANG);
                // 127(DEL)
                let is_del = (block ^ DEL).wrapping_sub(LSB);

                let result = (is_hs | lt_33 | is_del | block) & MSB;
                if result != 0 {
                    let nth = (result.trailing_zeros() / 8) as usize;
                    $cursor.advance(nth);
                    let $val = chunk[nth];
                    break 'swar $matches;
                }

                $cursor.advance(BLOCK);
            }

            while let Some($val) = $cursor.next() {
                simd::byte_map! {
                    const PAT =
                        #[default(true)]
                        #[false](b'!'..=b'~')
                        #[true](b'#')
                }

                if PAT[$val as usize] {
                    $cursor.step_back(1);
                    break 'swar $matches;
                }
            }
        }
    };
}

pub(crate) use {byte_map, validate_scheme, validate_authority, match_query, match_fragment};
