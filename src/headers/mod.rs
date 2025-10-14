//! HTTP Header Multimap.

// NOTE: this API DOES NOT handle comma separated values
// some headers just does not built for such specs
// notably headers with date value which requires comma
// spec: https://www.rfc-editor.org/rfc/rfc9110.html#name-field-lines-and-combined-fi

mod matches;
mod name;
mod value;
mod field;
mod map;
mod iter;

pub mod error;

pub use name::{HeaderName, AsHeaderName, IntoHeaderName, standard};
pub use value::{HeaderValue, InvalidHeaderValue};
pub use field::{HeaderField, GetAll};
pub use map::HeaderMap;
pub use iter::Iter;
