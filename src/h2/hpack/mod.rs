mod huffman;
mod table;

// generated code
mod huffman_table;
mod encode_table;

pub use table::{Table, Field, DecodeError};

#[cfg(test)]
mod test;
