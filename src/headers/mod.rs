//! HTTP Header Multimap.
mod matches;
mod name;
mod value;
mod entry;
mod map;
mod iter;

pub mod error;

pub use name::{HeaderName, AsHeaderName, IntoHeaderName, standard};
pub use value::{HeaderValue, InvalidHeaderValue};
pub use entry::{Entry, GetAll};
pub use map::HeaderMap;
pub use iter::Iter;
