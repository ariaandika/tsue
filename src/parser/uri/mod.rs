mod simd;

#[allow(clippy::module_inception)]
pub mod uri;
pub mod scheme;
pub mod path;
pub mod authority;
pub mod error;

#[cfg(test)]
mod test;
