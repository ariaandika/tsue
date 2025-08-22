use tcio::bytes::{ByteStr, Bytes};

#[derive(Debug)]
pub struct Authority {
    value: ByteStr,
    port: u16,
}

impl Authority {
    /// # Safety
    ///
    /// `host` must be valid ASCII
    pub(crate) unsafe fn new_unchecked(host: Bytes, port: u16) -> Self {
        Self {
            // SAFETY: ensured by caller
            value: unsafe { ByteStr::from_utf8_unchecked(host) },
            port,
        }
    }

    #[inline]
    pub const fn host(&self) -> &str {
        self.value.as_str()
    }

    #[inline]
    pub const fn port(&self) -> u16 {
        self.port
    }
}
