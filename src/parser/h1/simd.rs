use tcio::bytes::Cursor;

pub const BLOCK: usize = size_of::<usize>();
pub const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
pub const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

pub const CR: usize = usize::from_ne_bytes([b'\r'; BLOCK]);
pub const LF: usize = usize::from_ne_bytes([b'\n'; BLOCK]);
pub const COLON: usize = usize::from_ne_bytes([b':'; BLOCK]);

macro_rules! block_eq {
    ($block:expr, $target:expr) => {{
        let is = $block ^ $target;
        is.wrapping_sub(LSB) & !is
    }};
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

                // '\r'
                let is_cr = block ^ CR;
                let is_cr = is_cr.wrapping_sub(LSB) & !is_cr;

                // '\n'
                let is_lf = block ^ LF;
                let is_lf = is_lf.wrapping_sub(LSB) & !is_lf;

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

/// Returns the colon index, if its returns 0, either an error or empty method.
///
/// Check the result of `cursor.next()`, may returns '\r', '\n', invalid character or None.
///
/// Invalid character: `byte >= 128`
#[must_use]
pub fn match_header(cursor: &mut Cursor) -> usize {
    while let Some(chunk) = cursor.peek_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);

        // look for ':'
        let is_colon = block_eq!(block, COLON);
        // look for '\r'
        let is_cr = block_eq!(block, CR);
        // look for '\n'
        let is_lf = block_eq!(block, LF);

        let result = (is_colon | is_cr | is_lf | block) & MSB;
        if result != 0 {
            let sp = (result.trailing_zeros() / 8) as usize + cursor.steps();

            match_crlf_inner(block, cursor);

            return sp;
        }

        cursor.advance(BLOCK);
    }

    while let Some(mut byte) = cursor.next() {
        if matches!(byte, b':' | b'\r' | b'\n') || byte >= 128 {
            let col = cursor.steps();

            loop {
                if matches!(byte, b'\r' | b'\n') || byte >= 128 {
                    break;
                }
                match cursor.next() {
                    Some(next) => byte = next,
                    None => break,
                }
            }

            cursor.step_back(1);
            return col;
        }
    }

    0
}

fn match_crlf_inner(mut block: usize, cursor: &mut Cursor) {
    loop {
        // look for '\r'
        let is_cr = block_eq!(block, CR);
        // look for '\n'
        let is_lf = block_eq!(block, LF);

        let result = (is_cr | is_lf | block) & MSB;
        if result != 0 {
            // this will not eat the matched character
            // `cursor.next()` will emit the character
            cursor.advance((result.trailing_zeros() / 8) as usize);
            return;
        }

        cursor.advance(BLOCK);

        match cursor.peek_chunk::<BLOCK>() {
            Some(next) => block = usize::from_ne_bytes(*next),
            None => break,
        }
    }

    while let Some(byte) = cursor.next() {
        if matches!(byte, b'\r' | b'\n') || byte >= 128 {
            cursor.step_back(1);
            return;
        }
    }
}


pub(crate) use {block_eq, match_crlf};
