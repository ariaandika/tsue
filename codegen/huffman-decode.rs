// This implementation is highly inspired by the h2 crate
//
// in the future it will be interesting to explore different approaches
use huffman::{source_iter, Bits};

fn main() {
    let decode = gen_decode_table();
    if matches!(std::env::var("PRETTY").ok().as_deref(), Some("1")) {
        print_decode_pretty(decode);
    } else {
        print_decode(decode);
    }
}

#[derive(Debug)]
struct Lookup {
    untagged_bit_mask: u8,
    shifts: usize,
    next: usize,
}

/// Decode table can be thought as a graph, where each node (represented as entry), have 16 edges.
/// Each edge is assigned with an ID from 0 to 15 (represented as index in an entry).
///
/// Decoding process starts by dividing input bytes into 4 bits. From initial node, these bits
/// (which represent a number from 0 to 15) is used to choose an edge to traverse the graph from
/// node to node. For each node, there will be flag that denote whether a character has been
/// decoded.
type DecodeTable = Vec<[DecodeEntry; 16]>;

/// Generate the decode table.
///
/// The first phase, just by iterating from the source string, basic entries can be created.
///
/// But not all bits is exactly at multiple of 4. This represent an intersection between 2 encoded
/// character. The second character bits is represented as **shifted** bits.
///
/// ```not_rust
/// first char bits
/// \
///  1 001
///    --- second char bits (shifted by 1)
/// ```
///
/// The second phase, is to generate all the characters bits again, shifted by 1 to 3. But
/// generation is not starting from the initial node, instead it starts from partial bits node in
/// the last character bits node that is not multiply by 4, which can be multiple nodes connected
/// to the same shifted bits node.
///
/// The current strategy is to track all the last partial bits node in separate list, traverse the
/// node again, and generation can be decided by whether current shifted bits has been generated or
/// not.
fn gen_decode_table() -> DecodeTable {
    let mut decode: DecodeTable = vec![DecodeEntry::new_entries()];
    let mut lookup = vec![];

    // first phase
    for source_line in source_iter() {
        let byte = source_line.byte();
        let bits = source_line.bits();
        let data = EntryData { byte, shifts: 0 };
        let initial = 0;
        process_decode_entry(data, bits, initial, &mut decode, &mut lookup);
    }

    // second phase
    for shift_bits in [b"0", b"00", b"000"] as [&[u8]; _] {
        for source_line in source_iter() {
            let byte = source_line.byte();
            let shifts = shift_bits.len();
            let data = EntryData { byte, shifts };

            let mut bits = source_line.bits().into_shifted(shift_bits);
            let untagged_bit_mask = bits.take_4() & (0b1111 >> shifts);

            // generate shifted bits entry only when it is tracked in lookup table

            for l in &lookup {
                if l.shifts == shifts && l.untagged_bit_mask == untagged_bit_mask {
                    process_decode_entry(data, bits, l.next, &mut decode, &mut lookup);
                    break;
                }
            }
        }
    }

    decode
}

fn process_decode_entry(
    data: EntryData,
    mut bits_iter: Bits,
    starting_index: usize,
    decode: &mut Vec<[DecodeEntry; 16]>,
    lookup: &mut Vec<Lookup>,
) {
    // entry generation not always start from first entry, because shifted bits is connected to
    // partial bits entry, not first entry
    let mut current_index = starting_index;

    while bits_iter.remaining() >= 4 {
        let remaining = bits_iter.remaining();
        let id = bits_iter.take_4();
        let maybe_eos = id == 0b1111;
        let new_index = decode.len();
        let entry = &mut decode[current_index][id as usize];

        if remaining == 4 {
            // total bits length is multiple of 4, no partial bits
            // - is eos
            // - next node is initial node
            entry.set(DecodeEntry::decoded(data, true, 0, 0b1111));
            return;
        }

        if let DecodeEntry::Some { next, .. } = entry {
            // current entry has been generated, just traverse the entry
            current_index = *next;
            continue;
        }

        // current entry has not been generated
        entry.set(DecodeEntry::some(data, maybe_eos, new_index));
        decode.push(DecodeEntry::new_entries());
        current_index = new_index;
    }

    // ===== padded / partial bits =====

    // last bits is partial, thus it also contains partial shifted bits from other character, the
    // goal here is to iterate to all posssible partial shifted bits, and put it to `lookup` for
    // later shifted bits entry generation

    let remaining = bits_iter.remaining();
    let last_id = bits_iter.take_4();
    let eos_bit_mask = 0b1111 >> remaining;

    for shifted_id in 0..1 << (4 - remaining) {
        let id = shifted_id | last_id;

        let next;
        let shifts = remaining;
        let untagged_bit_mask = id & eos_bit_mask;
        let maybe_eos = (id & eos_bit_mask) == eos_bit_mask;
        let tagged_bit_mask = !eos_bit_mask & 0b1111;

        // multiple partial bits entry can point to the same shifted bits entry, if the shifted
        // entry already created, just point to it
        let lookup_entry = lookup.iter().find(|lookup|{
            lookup.untagged_bit_mask == untagged_bit_mask
            && lookup.shifts == shifts
        });
        match lookup_entry {
            Some(e) => next = e.next,
            None => {
                next = decode.len();
                lookup.push(Lookup {
                    untagged_bit_mask,
                    shifts,
                    next,
                });
                decode.push(DecodeEntry::new_entries());
            },
        }

        // all partial bits is decoded entry
        decode[current_index][id as usize].set(DecodeEntry::decoded(
            data,
            maybe_eos,
            next,
            tagged_bit_mask,
        ));
    }
}

fn print_decode(decode: DecodeTable) {
    println!("// autogenerated by codegen/huffman-code.rs");
    println!("#![cfg_attr(rustfmt, rustfmt_skip)]");
    println!("pub const DECODE_TABLE:[[(u8,u8,u8);16];256]=[");

    for entries in decode {
        print!("[");
        for entry in entries {
            match entry {
                DecodeEntry::None => {
                    print!("(0,0,0b100),");
                }
                DecodeEntry::Some { next, maybe_eos, .. } => {
                    print!("({},0,0b0{}0),", next, maybe_eos as u8);
                }
                DecodeEntry::Decoded { byte, maybe_eos, next, .. } => {
                    print!("({next},{byte},0b0{}1),", maybe_eos as u8);
                }
                DecodeEntry::Error => unreachable!(),
            }
        }
        println!("],");
    }
    println!("];");
}

fn print_decode_pretty(decode: DecodeTable) {
    println!("// (next id, byte, flags[error, maybe_eos, is_decoded])");
    println!("pub const DECODE_TABLE: [[(u8, u8, u8); 16]; 256] = [");

    for (i, entries) in decode.into_iter().enumerate() {
        println!("    [ // {i}");
        for (i, entry) in entries.into_iter().enumerate() {
            match entry {
                DecodeEntry::None => {
                    println!("        (  0,    0, 0b100), // ERROR");
                }
                DecodeEntry::Some { next, .. } => {
                    println!("        ({: >3},    0, 0b000), // {:0>4b} Partial", next, i);
                }
                DecodeEntry::Decoded { byte, maybe_eos, next, tagged_bit_mask: t, .. } => {
                    print!("        ({: >3}, ",next);
                    if byte.is_ascii_graphic() {
                        print!("b'{}', ",byte.escape_ascii());
                    } else {
                        print!("{byte: >4}, ");
                    }
                    print!("0b0{}", maybe_eos as u8);
                    println!("1), // {:0>4b} Decoded({t:0>4b})", i);
                }
                DecodeEntry::Error => unreachable!(),
            }
        }
        println!("    ],");
    }

    println!("];");
}

// ===== State =====

#[derive(Clone, Debug)]
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
