use tcio::bytes::{ByteStr, Bytes};

#[derive(Debug, Clone)]
pub struct Authority {
    value: Option<ByteStr>,
    port: u16,
}

impl Authority {
    pub(crate) const fn none() -> Authority {
        Self {
            value: None,
            port: 0,
        }
    }

    /// # Safety
    ///
    /// `host` must be valid ASCII
    pub(crate) unsafe fn new_unchecked(host: Bytes, port: u16) -> Self {
        Self {
            // SAFETY: ensured by caller
            value: Some(unsafe { ByteStr::from_utf8_unchecked(host) }),
            port,
        }
    }

    #[inline]
    pub const fn host(&self) -> Option<&str> {
        match &self.value {
            Some(host) => Some(host.as_str()),
            None => None,
        }
    }

    #[inline]
    pub const fn port(&self) -> u16 {
        self.port
    }
}

impl PartialEq for Authority {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
