//! HTTP Header Multimap.
mod name;
mod value;
mod entry;
mod map;
mod iter;

pub use name::{HeaderName, AsHeaderName, IntoHeaderName};
pub use value::{HeaderValue, InvalidHeaderValue};
pub use map::HeaderMap;
pub use entry::{Entry, GetAll};
pub use iter::Iter;
