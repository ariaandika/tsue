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

    pub fn bits(&self) -> impl Iterator<Item = bool> {
        self.line[super::BITS_RANGE].iter().filter_map(|b| match b {
            b'0' | b'1' => Some(matches!(b, b'1')),
            b'|' | b' ' => None,
            _ => panic!("invalid byte in bit column"),
        })
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
