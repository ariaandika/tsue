mod simd;

mod request;
mod header;
mod error;

pub use request::{Reqline, Target, Kind};
pub use header::Header;
pub use error::Error;

#[cfg(test)]
mod test;
