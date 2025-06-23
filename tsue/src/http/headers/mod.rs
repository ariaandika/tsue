//! HTTP Header Multimap.
mod name;
mod value;
mod entry;
mod map;
mod iter;

pub use name::{HeaderName, AsHeaderName};
pub use value::{HeaderValue, Sequence};
pub use map::HeaderMap;
pub use entry::{Entry, GetAll};
pub use iter::Iter;
