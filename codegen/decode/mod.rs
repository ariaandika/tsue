
#[derive(Clone, Debug)]
pub struct Meta {
    byte: u8,
}

#[derive(Clone, Debug)]
pub enum DecodeEntry {
    None,
    Partial {
        meta: Meta,
        next: Option<usize>,
    },
    Decoded {
        meta: Meta,
        byte: u8,
        is_maybe_eos: bool,
        next: Option<usize>,
    },
    Error,
}

const FLAG_MAYBE_EOS: u8 = 0b001;
const FLAG_DECODED: u8 = 0b010;
const FLAG_ERROR: u8 = 0b100;

impl Meta {
    pub const fn new(byte: u8) -> Self {
        Self { byte }
    }

    pub const fn none() -> Self {
        Self { byte: u8::MAX }
    }
}

impl DecodeEntry {
    pub const fn none() -> Self {
        Self::None
    }

    pub const fn partial(meta: Meta, next: Option<usize>) -> Self {
        Self::Partial {
            meta,
            next,
        }
    }

    pub const fn decoded(byte: u8, is_maybe_eos: bool, next: Option<usize>) -> Self {
        Self::Decoded {
            meta: Meta::none(),
            byte,
            is_maybe_eos,
            next,
        }
    }

    pub fn new_entries() -> [Self; 16] {
        [const { Self::none() }; 16]
    }

    pub fn set(&mut self, me: Self) {
        match &self {
            Self::None => *self = me,
            _ => panic!("duplicate assignment on DecodeEntry: {self:?} for {me:?}"),
        }
    }
}
