//! HTTP Header Multimap.
//!
//! This module contains the [`HeaderMap`] type, various HTTP header related types, and an error
//! that can occur during header related operations.
//!
//! # Examples
//!
//! ```
//! # struct User;
//! # impl User {
//! #     fn cookie(&self) -> String { "c".to_string() }
//! #     fn cookie2(&self) -> String { "c1".to_string() }
//! # }
//! # let user = User;
//! use tsue::headers::{
//!     HeaderMap,
//!     standard::{CONTENT_TYPE, CONTENT_LENGTH, COOKIE},
//!     HeaderValue,
//! };
//!
//! let mut map = HeaderMap::new();
//!
//! // insert header
//! map.insert(CONTENT_TYPE, HeaderValue::from_static(b"text/html"));
//! map.insert(CONTENT_LENGTH, HeaderValue::from_static(b"128"));
//!
//! // header lookup
//! assert!(map.contains_key(CONTENT_LENGTH));
//! assert_eq!(map.get(CONTENT_LENGTH), Some(&HeaderValue::from_static(b"128")));
//!
//! // `HeaderMap` is a multimap
//! let cookie: String = user.cookie();
//! let cookie_hdr = HeaderValue::from_string(cookie);
//! let cookie2: String = user.cookie2();
//! let cookie2_hdr = HeaderValue::from_string(cookie2);
//!
//! // use `append` to have multiple values in the same key
//! map.append(COOKIE, cookie_hdr.clone());
//! map.append(COOKIE, cookie2_hdr.clone());
//!
//! // retrieve all values from single key in insertion order
//! let mut values = map.get_all(COOKIE);
//! assert_eq!(values.next(), Some(&cookie_hdr));
//! assert_eq!(values.next(), Some(&cookie2_hdr));
//! assert!(values.next().is_none());
//!
//! // use `insert` to replace existing value for given key
//! let old_type = map.insert(CONTENT_TYPE, HeaderValue::from_static(b"text/plain"));
//! assert_eq!(old_type, Some(HeaderValue::from_static(b"text/html")));
//! ```

// NOTE: this API DOES NOT handle comma separated values
// some headers just does not built for such specs
// notably headers with date value which requires comma
// spec: https://www.rfc-editor.org/rfc/rfc9110.html#name-field-lines-and-combined-fi

// NOTE: should header value be limited to US-ASCII only ?

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
pub mod iter;

#[cfg(test)]
mod test;

pub mod error;

pub use name::{HeaderName, standard};
pub use value::HeaderValue;
pub use field::HeaderField;
pub use map::{HeaderMap, AsHeaderName, IntoHeaderName};
