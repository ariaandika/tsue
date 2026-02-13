/// HTTP/2 Frame Type.
#[derive(Debug, Clone, Copy)]
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

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Data => "DATA",
            Self::Headers => "HEADERS",
            Self::Priority => "PRIORITY",
            Self::RstStream => "RST_STREAM",
            Self::Settings => "SETTINGS",
            Self::PushPromise => "PUSH_PROMISE",
            Self::Ping => "PING",
            Self::GoAway => "GOAWAY",
            Self::WindowUpdate => "WINDOW_UPDATE",
            Self::Continuation => "CONTINUATION",
        }
    }
}

/// HTTP/2 Frame Header.
#[derive(Clone ,Debug)]
pub struct Header {
    /// The length of the frame payload expressed as an unsigned 24-bit integer in units of octets.
    ///
    /// Values greater than 2^14 (16,384) MUST NOT be sent unless the receiver has set a larger
    /// value for SETTINGS_MAX_FRAME_SIZE.
    ///
    /// The 9 octets of the frame header are not included in this value.
    pub len: u32,
    /// The 8-bit type of the frame.
    ///
    /// The frame type determines the format and semantics of the frame. Implementations MUST
    /// ignore and discard frames of unknown types.
    pub ty: u8,
    /// An 8-bit field reserved for boolean flags specific to the frame type.
    ///
    /// Flags are assigned semantics specific to the indicated frame type. Unused flags are those
    /// that have no defined semantics for a particular frame type. Unused flags MUST be ignored on
    /// receipt and MUST be left unset (0x00) when sending.
    pub flags: u8,
    // /// A reserved 1-bit field.
    // ///
    // /// The semantics of this bit are undefined, and the bit MUST remain
    // /// unset (0x00) when sending and MUST be ignored when receiving.
    // pub reserved: bool,
    /// A stream identifier expressed as an unsigned 31-bit integer.
    ///
    /// The value 0x00 is reserved for frames that are associated with the connection as a whole as
    /// opposed to an individual stream.
    pub stream_id: u32,
}

impl Header {
    /// Length of encoded frame header bytes.
    pub(crate) const SIZE: usize = 9;
    pub(crate) const EMPTY_SETTINGS: [u8; 9] = [0, 0, 0, 4, 0, 0, 0, 0, 0];
    pub(crate) const ACK_SETTINGS: [u8; 9] = [0, 0, 0, 4, 1, 0, 0, 0, 0];
    pub(crate) const ACK_PING: [u8; 9] = [0, 0, 8, 6, 1, 0, 0, 0, 0];

    pub(crate) fn frame_type_of(chunk: &[u8; Self::SIZE]) -> Option<Type> {
        Type::from_u8(chunk[3])
    }

    pub(crate) fn decode(bytes: &[u8; Self::SIZE]) -> Self {
        // Length (24),
        // Type (8),
        // Flags (8),
        // Reserved (1),
        // Stream Identifier (31),

        let len = u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]);
        let ty = bytes[3];
        let flags = bytes[4];
        // let reserved = bytes[5] & 0b10000000 != 0;
        let stream_id = u32::from_be_bytes([bytes[5] & 0b01111111, bytes[6], bytes[7], bytes[8]]);

        Header {
            len,
            ty,
            flags,
            // reserved,
            stream_id,
        }
    }

    pub fn encode(self) -> [u8; 9] {
        let mut buffer = [0u8; 10];
        buffer[..4].copy_from_slice(&self.len.to_be_bytes());
        buffer[4] = self.ty;
        buffer[5] = self.flags;
        buffer[6..].copy_from_slice(&(self.stream_id & u32::MAX >> 1).to_be_bytes());
        buffer[1..].try_into().unwrap()
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub const fn frame_size(&self) -> usize {
        Self::SIZE + self.len as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn frame_type(&self) -> Option<Type> {
        Type::from_u8(self.ty)
    }
}

