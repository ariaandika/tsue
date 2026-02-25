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

/// Returns `true` if byte is valid header name.
///
/// field-value    = *field-content
/// field-content  = field-vchar
///                  [ 1*( SP / HTAB / field-vchar ) field-vchar ]
/// field-vchar    = VCHAR / obs-text
/// obs-text       = %x80-FF
///
/// Note, `obs-text` is NOT supported.
#[inline(always)]
pub const fn is_header_value(byte: u8) -> bool {
    const fn valid(byte: u8) -> bool {
        // VCHAR                    || SP / HTAB
        matches!(byte, 0x21..=0x7E) || matches!(byte, b' ' | b'\t')
    }

    const PAT: [bool; 256] = {
        let mut bytes = [false; 256];
        let mut byte = 0u8;
        loop {
            bytes[byte as usize] = valid(byte);
            // 127 > is non-ascii
            if byte == 127 {
                break;
            }
            byte += 1;
        }
        bytes
    };

    PAT[byte as usize]
}

