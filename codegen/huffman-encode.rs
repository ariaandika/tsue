// This implementation is highly inspired by the h2 crate
//
// in the future it will be interesting to explore different approaches
use huffman::source_iter;
mod huffman;

fn main() {
    print_encode_table();
}

fn print_encode_table() {
    println!("// (bits_len, bits)");
    print!("pub const ENCODE_TABLE: [(u8, u32); 256] = [");
    for source_line in source_iter() {
        let bits_iter = source_line.bits();
        let len = bits_iter.remaining();
        let bits = bits_iter.fold(0u32, |acc, next| (acc << 1) | next as u32);
        // let bytes = bits.to_le_bytes();
        print!("(0x{len:X},0x{bits:X}),");
    }
    print!("];");
}
