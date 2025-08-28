mod simd;

#[allow(clippy::module_inception)]
mod uri;
mod scheme;
mod path;
mod authority;
mod error;

pub use uri::{parse, Target};
pub use scheme::Scheme;
pub use path::Path;
pub use authority::Authority;
pub use error::UriError;

#[cfg(test)]
mod test;
