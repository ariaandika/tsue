mod huffman;
mod table;
mod decoder;
mod encoder;

// generated code
mod decode_table;
mod encode_table;

pub use table::Table;
pub use decoder::{Decoder, DecodeError};
pub use encoder::Encoder;

#[cfg(test)]
mod test;
