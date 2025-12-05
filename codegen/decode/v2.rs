

#[derive(Debug)]
pub struct DecodeTree {
    entries: Vec<DecodeParts>,
}

#[derive(Debug)]
pub struct DecodeParts {
    entries: [DecodeEntry; 16],
}

pub struct DecodeEntry {
    byte: u8,
    flags: u8,
    next: Option<Box<DecodeParts>>,
    kind: Kind,
}

impl std::fmt::Debug for DecodeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodeEntry")
            .field("byte", &(self.byte as char))
            .field("flags", &self.flags)
            .field("next", &self.next)
            .field("kind", &self.kind)
            .finish()
    }
}

#[derive(Debug)]
enum Kind {
    Error,
    Decoded,
    Partial,
}

impl DecodeEntry {
    const fn new_error() -> Self {
        Self {
            byte: 0,
            flags: super::FLAG_ERROR,
            next: None,
            kind: Kind::Error,
        }
    }

    fn new_decoded(byte: u8, is_maybe_eos: bool) -> DecodeEntry {
        Self {
            byte,
            flags: super::FLAG_DECODED
                | if is_maybe_eos {
                    super::FLAG_MAYBE_EOS
                } else {
                    0
                },
            next: Some(Box::new(DecodeParts::new())),
            kind: Kind::Decoded,
        }
    }

    fn new_partial() -> DecodeEntry {
        Self {
            byte: 0,
            flags: 0b000,
            next: Some(Box::new(DecodeParts::new())),
            kind: Kind::Partial,
        }
    }

    /// Returns the next entry parts.
    pub fn as_partial(&mut self) -> &mut DecodeParts {
        if self.flags & super::FLAG_ERROR != 0 {
            *self = Self::new_partial();
        }
        self.next.as_mut().unwrap()
    }

    pub fn as_decoded(&mut self, byte: u8, is_maybe_eos: bool) {
        assert!(matches!(self.kind, Kind::Error), "{self:?}");
        assert!(self.flags & super::FLAG_ERROR != 0, "{self:?}");
        *self = Self::new_decoded(byte, is_maybe_eos);
        self.flags |= super::FLAG_DECODED;
    }
}

impl DecodeParts {
    pub fn new() -> Self {
        Self {
            entries: [const { DecodeEntry::new_error() }; 16],
        }
    }

    pub fn by_id_mut(&mut self, id: u8) -> &mut DecodeEntry {
        assert!(id & 0b1111_0000 == 0);
        &mut self.entries[id as usize]
    }
}

impl DecodeTree {
    pub fn new() -> Self {
        Self { entries: vec![] }
    }

    pub fn first_mut(&mut self) -> &mut DecodeParts {
        if self.entries.is_empty() {
            self.entries.push(DecodeParts::new());
        }
        self.entries.first_mut().unwrap()
    }
}

impl std::ops::Deref for DecodeTree {
    type Target = Vec<DecodeParts>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}
