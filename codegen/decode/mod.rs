
#[derive(Clone)]
pub enum DecodeEntry {
    None,
    Partial {
        next: usize,
        maybe_eos: bool,
        shifted: usize,
    },
    Decoded {
        byte: u8,
        shifted: usize,
        maybe_eos: bool,
        next: usize,
        tagged_bit_mask: u8,
    },
    #[allow(dead_code)]
    Error,
}

#[derive(Clone, Copy, Debug)]
pub struct EntryData {
    pub byte: u8,
    pub shifted: usize,
}

impl DecodeEntry {
    pub const fn none() -> Self {
        Self::None
    }

    pub fn partial(data: EntryData, maybe_eos: bool, next: usize) -> Self {
        Self::Partial {
            next,
            maybe_eos,
            shifted: data.shifted,
        }
    }

    pub const fn decoded(data: EntryData, maybe_eos: bool, next: usize, tagged_bit_mask: u8) -> Self {
        Self::Decoded {
            byte: data.byte,
            shifted: data.shifted,
            maybe_eos,
            next,
            tagged_bit_mask,
        }
    }

    pub const fn new_entries() -> [Self; 16] {
        [const { Self::none() }; 16]
    }

    /// Current entry must be `None`, otherwise panic.
    pub fn set(&mut self, me: Self) {
        assert!(
            matches!(self, Self::None),
            "duplicate assignment on DecodeEntry: {self:?} for {me:?}"
        );
        *self = me;
    }
}

impl std::fmt::Debug for DecodeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Partial {
                next,
                maybe_eos,
                shifted,
            } => f
                .debug_struct("Partial")
                .field("next", next)
                .field("maybe_eos", maybe_eos)
                .field("shifted", shifted)
                .finish(),
            Self::Decoded {
                byte,
                maybe_eos,
                next,
                tagged_bit_mask,
                shifted,
            } => f
                .debug_struct("Decoded")
                .field("byte", &(*byte as char))
                .field("maybe_eos", maybe_eos)
                .field("next", next)
                .field("tagged_bit_mask", &format_args!("{tagged_bit_mask:0>4b}"))
                .field("shifted", shifted)
                .finish(),
            Self::Error => write!(f, "Error"),
        }
    }
}

