pub mod v2;

pub struct DecodeTable {
    entries: Vec<[DecodeEntry; 16]>,
}

#[derive(Clone, Debug)]
pub struct DecodeEntry {
    next_state: Option<usize>,
    byte: u8,
    flags: u8,
    mutated: bool,
}

const FLAG_MAYBE_EOS: u8    = 0b001;
const FLAG_DECODED: u8      = 0b010;
const FLAG_ERROR: u8        = 0b100;

impl DecodeEntry {
    pub fn new_entries() -> [Self; 16] {
        let mut entries = Vec::with_capacity(16);
        for _ in 0..16 {
            entries.push(Self {
                next_state: None,
                byte: 0,
                flags: FLAG_ERROR,
                mutated: false,
            });
        }
        entries.try_into().unwrap()
    }

    pub fn next_state(&self) -> Option<usize> {
        self.next_state
    }

    pub fn next_state_mut(&mut self) -> &mut Option<usize> {
        &mut self.next_state
    }

    pub fn set_maybe_eos(&mut self) {
        self.mutated = true;
        self.flags |= FLAG_MAYBE_EOS;
    }

    pub fn set_byte(&mut self, byte: u8) {
        self.byte = byte;
    }

    pub fn set_decoded(&mut self, byte: u8) {
        self.mutated = true;
        self.flags |= FLAG_DECODED;
        self.byte = byte;
    }

    pub fn unset_error(&mut self) {
        self.mutated = true;
        self.flags &= !FLAG_ERROR;
    }
}

impl DecodeTable {
    pub fn new() -> Self {
        Self { entries: vec![DecodeEntry::new_entries()] }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn first_mut(&mut self) -> &mut [DecodeEntry; 16] {
        self.entries.first_mut().unwrap()
    }

    pub fn entries_mut(&mut self, state: usize) -> &mut [DecodeEntry; 16] {
        &mut self.entries[state]
    }

    pub fn push_entry(&mut self) {
        self.entries.push(DecodeEntry::new_entries());
    }

    pub fn debug_print(&self) {
        println!("[{}]",self.entries.len());
        for (i, entries) in self.entries.iter().enumerate()/* .skip(28) *//* .take(5) */ {
            println!("{i} [");
            for (i, DecodeEntry { next_state, byte, flags, mutated }) in entries.iter().enumerate() {
                print!("  [0b{:0>4b};{mutated}] next_state: {next_state:?},",i);
                print!("  flags: 0b{flags:0>3b}");
                print!("  byte: {:?},", *byte as char);
                println!();
            }
            println!("]");
        }
    }
}

