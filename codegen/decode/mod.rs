
#[derive(Clone)]
pub enum DecodeEntry {
    None,
    Some {
        next: usize,
        maybe_eos: bool,
        shifts: usize,
    },
    Decoded {
        next: usize,
        maybe_eos: bool,
        shifts: usize,
        byte: u8,
        tagged_bit_mask: u8,
    },
    #[allow(dead_code)]
    Error,
}

#[derive(Clone, Copy, Debug)]
pub struct EntryData {
    pub byte: u8,
    pub shifts: usize,
}

impl DecodeEntry {
    pub const fn none() -> Self {
        Self::None
    }

    pub fn some(data: EntryData, maybe_eos: bool, next: usize) -> Self {
        Self::Some {
            next,
            maybe_eos,
            shifts: data.shifts,
        }
    }

    pub const fn decoded(data: EntryData, maybe_eos: bool, next: usize, tagged_bit_mask: u8) -> Self {
        Self::Decoded {
            byte: data.byte,
            shifts: data.shifts,
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
            Self::Some {
                next,
                maybe_eos,
                shifts,
            } => f
                .debug_struct("Partial")
                .field("next", next)
                .field("maybe_eos", maybe_eos)
                .field("shifts", shifts)
                .finish(),
            Self::Decoded {
                byte,
                maybe_eos,
                next,
                tagged_bit_mask,
                shifts,
            } => f
                .debug_struct("Decoded")
                .field("byte", &(*byte as char))
                .field("maybe_eos", maybe_eos)
                .field("next", next)
                .field("tagged_bit_mask", &format_args!("{tagged_bit_mask:0>4b}"))
                .field("shifts", shifts)
                .finish(),
            Self::Error => write!(f, "Error"),
        }
    }
}

