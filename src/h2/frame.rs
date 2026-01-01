#![allow(unused, reason = "TODO: h2 implementation")]

/// Frame Type.
#[derive(Debug)]
#[repr(u8)]
pub enum Type {
    Data = 0,
    Headers = 1,
    /// DEPRECATED.
    Priority = 2,
    RstStream = 3,
    Settings = 4,
    PushPromise = 5,
    Ping = 6,
    GoAway = 7,
    WindowUpdate = 8,
    Continuation = 9,
}

impl Type {
    pub fn from_u8(ty: u8) -> Option<Self> {
        if ty < 10 {
            Some(unsafe { core::mem::transmute::<u8, Self>(ty) })
        } else {
            None
        }
    }
}

/// Frame Header.
#[derive(Debug)]
pub struct Header {
    pub len: u32,
    pub ty: u8,
    pub flags: u8,
    pub stream_id: u32,
}

impl Header {
    pub(crate) fn decode(bytes: [u8; 9]) -> Self {
        // Length (24),
        // Type (8),
        // Flags (8),
        // Reserved (1),
        // Stream Identifier (31),

        let len = u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]);
        let ty = bytes[3];
        let flags = bytes[4];
        let stream_id = u32::from_be_bytes([bytes[5] & 0b01111111, bytes[6], bytes[7], bytes[8]]);

        Header {
            len,
            ty,
            flags,
            stream_id,
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

