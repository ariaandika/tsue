pub(crate) use crate::matches::*;

byte_map! {
    /// field-name = token
    #[inline(always)]
    pub const fn is_field_name_char(byte: u8) {
        self::is_token(byte)
    }
}

