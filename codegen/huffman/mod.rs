pub struct Table {
    iter: Box<dyn Iterator<Item = &'static [u8]>>,
}

#[derive(Clone)]
pub struct TableEntry {
    line: &'static [u8],
}

impl TableEntry {
    fn new(line: &'static [u8]) -> Self {
        Self { line }
    }

    pub fn bits(&self) -> Bits<'_, impl ExactSizeIterator<Item = u8>> {
        Bits {
            remaining: self.bits_len(),
            inner: self.line[super::BITS_RANGE].iter().copied(),
            entry: self,
        }
    }

    pub fn bits_len(&self) -> usize {
        str::from_utf8(self.line[super::BITS_LEN_RANGE].trim_ascii_start())
            .unwrap()
            .parse()
            .unwrap()
    }

    /// Returns `None` for EOS entry.
    pub fn byte(&self) -> Option<u8> {
        match str::from_utf8(self.line[super::BYTE_RANGE].trim_ascii_start())
            .unwrap()
            .parse()
        {
            Ok(ok) => Some(ok),
            Err(err) => {
                if self.line[super::BYTE_RANGE].eq(b"256") {
                    // EOS
                    None
                } else {
                    panic!("invalid code from string source: {err}")
                }
            }
        }
    }
}

impl Table {
    pub fn new(source: &'static str) -> Self {
        Self {
            iter: Box::new(source.lines().skip(1).map(|e| e.as_bytes())),
        }
    }
}

impl Iterator for Table {
    type Item = TableEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(TableEntry::new)
    }
}

// ===== Bits =====

pub struct Bits<'a, T> {
    entry: &'a TableEntry,
    remaining: usize,
    inner: T,
}

impl<T: Iterator<Item = u8>> Bits<'_, T> {
    pub fn bits_len(&self) -> usize {
        self.entry.bits_len()
    }

    pub fn remaining(&self) -> usize {
        self.remaining
    }

    pub fn assert_4(&mut self) -> u8 {
        let mut bits = 0u8;
        bits |= (self.next().unwrap() as u8) << 3;
        bits |= (self.next().unwrap() as u8) << 2;
        bits |= (self.next().unwrap() as u8) << 1;
        bits |= self.next().unwrap() as u8;
        bits
    }

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
}

impl<T: Iterator<Item = u8>> Iterator for Bits<'_, T> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
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

