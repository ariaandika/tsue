use tcio::bytes::Cursor;

const BLOCK: usize = size_of::<usize>();
const MSB: usize = usize::from_ne_bytes([0b1000_0000; BLOCK]);
const LSB: usize = usize::from_ne_bytes([0b0000_0001; BLOCK]);

const COLON: usize = usize::from_ne_bytes([b':'; BLOCK]);
const BANG: usize = usize::from_ne_bytes([b'!'; BLOCK]);
const HASH: usize = usize::from_ne_bytes([b'#'; BLOCK]);
const SLASH: usize = usize::from_ne_bytes([b'/'; BLOCK]);
const QS: usize = usize::from_ne_bytes([b'?'; BLOCK]);
const DEL: usize = usize::from_ne_bytes([b'~' + 1; BLOCK]);

macro_rules! block_eq {
    ($block:expr, $target:expr) => {{
        let is = $block ^ $target;
        is.wrapping_sub(LSB) & !is
    }};
}

/// Inclusive
///
/// Only works for `target` less than 128.
#[allow(unused, reason = "for reference")]
macro_rules! block_lt {
    ($block:ident, $target:expr) => {
        $block.wrapping_sub($target) & !$block
    };
}

/// Inclusive
///
/// Only works for `target` less than 128, and guarantee that `block` MSB is unset.
///
/// Does not handle if the `block` MSB is already set.
///
/// The goal is to set MSB if `block` subtract wrapped.
///
/// But in the case of `block` MSB is set, and subtraction does not unset the MSB, it will returns
/// the invalid result.
macro_rules! block_lt_no_msb {
    ($block:ident, $target:expr) => {
        $block.wrapping_sub($target)
    };
}

/// Check the result of `cursor.next()`, may returns ':', invalid character, or None.
///
/// URI leader is for matching `scheme ":"` or `host ":"`
///
/// Invalid character: `matches!(byte, b'/') || !matches!(b'!'..=b'~')`
pub fn match_uri_leader(cursor: &mut Cursor) {
    while let Some(chunk) = cursor.peek_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);

        // look for ":"
        let is_colon = block_eq!(block, COLON);
        // look for "/"
        let is_sl = block_eq!(block, SLASH);
        // 33(BANG) <= byte < 127(DEL)
        // if MSB is set on `block`, value is >= 128
        let lt_33 = block_lt_no_msb!(block, BANG);
        let is_del = block_eq!(block, DEL);

        let result = (is_colon | is_sl | lt_33 | is_del | block) & MSB;
        if result != 0 {
            cursor.advance((result.trailing_zeros() / 8) as usize);
            return;
        }

        cursor.advance(BLOCK);
    }

    while let Some(byte) = cursor.next() {
        if matches!(byte, b':' | b'/') || !matches!(byte, b'!'..=b'~') {
            cursor.step_back(1);
            return;
        }
    }
}

/// Check the result of `cursor.next()`, may returns '?', '#', invalid character, or None.
///
/// Invalid character: `!matches!(b'!'..=b'~')`
pub fn match_path(cursor: &mut Cursor) {
    while let Some(chunk) = cursor.peek_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);

        // look for "?"
        let is_qs = block_eq!(block, QS);
        // look for "#"
        let is_hash = block_eq!(block, HASH);
        // 33(BANG) <= byte < 127(DEL)
        // if MSB is set on `block`, value is >= 128
        let lt_33 = block_lt_no_msb!(block, BANG);
        let is_del = block_eq!(block, DEL);

        let result = (is_qs | is_hash | is_del | lt_33 | block) & MSB;
        if result != 0 {
            cursor.advance((result.trailing_zeros() / 8) as usize);
            return;
        }

        cursor.advance(BLOCK);
    }

    while let Some(byte) = cursor.next() {
        if matches!(byte, b'?' | b'#') || !matches!(byte, b'!'..=b'~') {
            cursor.step_back(1);
            return;
        }
    }

    // contains full path
}

/// Check the result of `cursor.next()`, may returns '#', invalid character, or None.
///
/// Invalid character: `!matches!(b'!'..=b'~')`
pub fn match_fragment(cursor: &mut Cursor) {
    while let Some(chunk) = cursor.peek_chunk::<BLOCK>() {
        let block = usize::from_ne_bytes(*chunk);

        // look for "#"
        let is_hash = block_eq!(block, HASH);
        // 33(BANG) <= byte < 127(DEL)
        // if MSB is set on `block`, value is >= 128
        let lt_33 = block_lt_no_msb!(block, BANG);
        let is_del = block_eq!(block, DEL);

        let result = (is_hash | is_del | lt_33 | block) & MSB;
        if result != 0 {
            cursor.advance((result.trailing_zeros() / 8) as usize);
            return;
        }

        cursor.advance(BLOCK);
    }

    while let Some(byte) = cursor.next() {
        if matches!(byte, b'#') || !matches!(byte, b'!'..=b'~') {
            cursor.step_back(1);
            return;
        }
    }

    // contains full path
}

