//! HTTP/1.1 Protocol.
mod parser;
mod proto;
mod conn;

#[cfg(test)]
mod test;

pub use conn::Connection;
