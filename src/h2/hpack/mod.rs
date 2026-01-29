mod huffman;
mod table;

// generated code
mod decode_table;
mod encode_table;

pub use table::{Table, DecodeError};

#[cfg(test)]
mod test;
