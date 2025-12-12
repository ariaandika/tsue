pub struct TableParser {
    iter: Box<dyn Iterator<Item = &'static [u8]>>,
}

#[derive(Clone)]
pub struct TableEntry {
    byte: u8,
    line: &'static [u8],
}

impl TableEntry {
    fn new(line: &'static [u8]) -> Option<Self> {
        match str::from_utf8(line[super::BYTE_RANGE].trim_ascii_start())
            .unwrap()
            .parse()
        {
            Ok(byte) => Some(Self { byte, line }),
            Err(err) => {
                if line[super::BYTE_RANGE].eq(b"256") {
                    // EOS
                    None
                } else {
                    panic!("invalid code from string source: {err}")
                }
            }
        }
    }

    pub fn bits(&self) -> Bits<'_, impl Iterator<Item = u8>> {
        Bits {
            remaining: self.bits_len(),
            iter: self.line[super::BITS_RANGE].iter().copied(),
            entry: self,
        }
    }

    pub fn bits_len(&self) -> usize {
        str::from_utf8(self.line[super::BITS_LEN_RANGE].trim_ascii_start())
            .unwrap()
            .parse()
            .unwrap()
    }

    pub fn byte(&self) -> u8 {
        self.byte
    }
}

impl TableParser {
    pub fn new(source: &'static str) -> Self {
        Self {
            iter: Box::new(source.lines().skip(1).map(|e| e.as_bytes())),
        }
    }
}

impl Iterator for TableParser {
    type Item = TableEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().and_then(TableEntry::new)
    }
}

// ===== Bits =====

/// Iterator over the bits in an entry.
pub struct Bits<'a, T> {
    entry: &'a TableEntry,
    remaining: usize,
    iter: T,
}

impl<'a, T: Iterator<Item = u8>> Bits<'a, T> {
    /// Note that remaining is including the shifted bit, not from the original length.
    pub fn remaining(&self) -> usize {
        self.remaining
    }

    /// This can returns less than 4 bit, and padded 4 to the left.
    pub fn take_4(&mut self) -> u8 {
        let mut id = 0u8;
        if let Some(bit) = self.next() {
            id |= (bit as u8) << 3;
        }
        if let Some(bit) = self.next() {
            id |= (bit as u8) << 2;
        }
        if let Some(bit) = self.next() {
            id |= (bit as u8) << 1;
        }
        if let Some(bit) = self.next() {
            id |= bit as u8;
        }
        id
    }

    pub fn shifted(self, shift: &'a [u8]) -> Bits<'a, impl Iterator<Item = u8>> {
        Bits {
            remaining: shift.len() + self.remaining,
            iter: shift.iter().copied().chain(self.iter),
            entry: self.entry,
        }
    }
}

impl<T: Iterator<Item = u8>> Iterator for Bits<'_, T> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(b @ (b'0' | b'1')) => {
                self.remaining = self.remaining.strict_sub(1);
                Some(matches!(b, b'1'))
            }
            Some(b'|' | b' ') => self.next(),
            Some(_) => panic!("invalid byte in bit column"),
            None => {
                assert_eq!(self.remaining, 0);
                None
            },
        }
    }
}

