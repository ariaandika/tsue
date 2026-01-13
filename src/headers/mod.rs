//! HTTP Header Multimap.

// NOTE: this API DOES NOT handle comma separated values
// some headers just does not built for such specs
// notably headers with date value which requires comma
// spec: https://www.rfc-editor.org/rfc/rfc9110.html#name-field-lines-and-combined-fi

// NOTE: should header value be limited to US-ASCII only ?

// NOTE: maintaining fields order, double allocation strategy where one store hash and index to the
// other that maintains the order

// NOTE: current header map optimization such as robin hood hashing or using cryptographic hash
// function is not implemented, as it is expected that user limit the header length to much lower
// number than the hard limit

// NOTE: current it is not possible to provide generic for custom hasher because a const evaluation
// is needed to compute a static header hash at compile time

mod matches;
mod name;
mod value;
mod field;
mod map;
mod iter;

#[cfg(test)]
mod test;

pub mod error;

pub use name::{HeaderName, standard};
pub use value::HeaderValue;
pub use field::{HeaderField, GetAll};
pub use map::{HeaderMap, AsHeaderName, IntoHeaderName};
pub use iter::Iter;
