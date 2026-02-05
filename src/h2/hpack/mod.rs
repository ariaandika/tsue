mod huffman;
mod repr;
mod table;
mod decoder;
mod encoder;

// generated code
mod decode_table;
mod encode_table;

pub use table::Table;
pub use decoder::Decoder;
pub use encoder::Encoder;

pub mod error;

#[cfg(test)]
mod test;
