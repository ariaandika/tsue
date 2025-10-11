macro_rules! byte_map {
    {
        $(#[$meta:meta])*
        $vis:vis const $cnid:ident =
            #[default($def:literal)]
            $(#[false]($nepat:pat))?
            $(#[true]($pat:pat))?
    } => {
        $(#[$meta])*
        $vis const $cnid: [bool; 256] = {
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
    // ===== 128 lookup table, usefull for ASCII byte =====
    {
        #[table_128]
        $(#[$meta:meta])*
        $vis:vis const unsafe fn $fn_id:ident($byte:ident:$u8:ty) { $e:expr }
    } => {
        $(#[$meta])*
        /// # Safety
        ///
        /// `byte` must be less than 128.
        $vis const unsafe fn $fn_id($byte: $u8) -> bool {
            static PAT: [bool; 128] = {
                let mut bytes = [false; 128];
                let mut $byte = 0u8;
                const fn filter($byte: $u8) -> bool {
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
            debug_assert!(byte < 128);
            // SAFETY: caller must ensure that `byte` is less than 128
            unsafe { *PAT.as_ptr().add($byte as usize) }
        }
    };
    // ===== 256 lookup table =====
    {
        $(#[$meta:meta])*
        $vis:vis const fn $fn_id:ident($byte:ident:$u8:ty) { $e:expr }
    } => {
        $(#[$meta])*
        $vis const fn $fn_id($byte: $u8) -> bool {
            static PAT: [bool; 256] = {
                let mut bytes = [false; 256];
                let mut $byte = 0u8;
                const fn filter($byte: $u8) -> bool {
                    $e
                }
                loop {
                    bytes[$byte as usize] = filter($byte);
                    if $byte == 255 {
                        break;
                    }
                    $byte += 1;
                }
                bytes
            };
            // SAFETY: the pattern size is equal to u8::MAX
            unsafe { *PAT.as_ptr().add($byte as usize) }
        }
    };
}

pub(crate) use {byte_map};

// ===== Blocks =====

byte_map! {
    /// unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"
    #[inline(always)]
    const fn unreserved(byte: u8) {
        byte.is_ascii_alphanumeric()
        || matches!(byte, b'-' | b'.' | b'_' | b'~')
    }
}

byte_map! {
    /// sub-delims = "!" / "$" / "&" / "'" / "(" / ")"
    ///            / "*" / "+" / "," / ";" / "="
    #[inline(always)]
    const fn sub_delims(byte: u8) {
        matches!(
            byte,
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
        )
        || byte.is_ascii_alphanumeric()
    }
}

byte_map! {
    /// pchar = unreserved / pct-encoded / sub-delims / ":" / "@"
    #[inline(always)]
    const fn is_pchar(byte: u8) {
        unreserved(byte)
        || matches!(byte, b'%')
        || sub_delims(byte)
        || matches!(byte, b':' | b'@')
    }
}

byte_map! {
    /// token   = 1*tchar
    /// tchar   = "!" / "#" / "$" / "%" / "&" / "'" / "*"
    ///         / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
    ///         / DIGIT / ALPHA
    #[inline(always)]
    pub const fn is_token(byte: u8) {
        matches!(
            byte,
            | b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*'
            | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
        )
        || byte.is_ascii_alphanumeric()
    }
}

// ===== lookup table =====

byte_map! {
    /// method  = token
    #[inline(always)]
    pub const fn is_method(byte: u8) {
        is_token(byte)
    }
}

byte_map! {
    /// scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    #[inline(always)]
    pub const fn is_scheme(byte: u8) {
        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')
    }
}

byte_map! {
    /// userinfo = *( unreserved / pct-encoded / sub-delims / ":" )
    #[inline(always)]
    pub const fn is_userinfo(byte: u8) {
        unreserved(byte)
        || matches!(byte, b'%')
        || byte.is_ascii_hexdigit()
        || sub_delims(byte)
        || matches!(byte, b':')
    }
}

byte_map! {
    /// hex / ":" / "."
    ///
    /// this is temporary until ipv6 validation is implemented
    #[inline(always)]
    pub const fn is_ipv6(byte: u8) {
        byte.is_ascii_hexdigit() || matches!(byte, b':' | b'.')
    }
}

byte_map! {
    /// reg-name = *( unreserved / sub-delims / ":" )
    #[inline(always)]
    pub const fn is_ipvfuture(byte: u8) {
        unreserved(byte) || sub_delims(byte) || matches!(byte, b':')
    }
}

byte_map! {
    /// reg-name = *( unreserved / pct-encoded / sub-delims )
    #[inline(always)]
    pub const fn is_regname(byte: u8) {
        unreserved(byte) || matches!(byte, b'%') || byte.is_ascii_hexdigit() || sub_delims(byte)
    }
}

byte_map! {
    /// segment         = *pchar
    /// path-abempty    = *( "/" / segment )
    #[inline(always)]
    pub const fn is_path(byte: u8) {
        is_pchar(byte) || matches!(byte, b':' | b'@' | b'/')
    }
}

byte_map! {
    /// query = *( pchar / "/" / "?" )
    #[inline(always)]
    pub const fn is_query(byte: u8) {
        is_pchar(byte) || matches!(byte, b'/' | b'?')
    }
}
