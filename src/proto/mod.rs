//! HTTP Semantics ([RFC9110]).
//!
//! This module contains types that represent HTTP Semantics.
//!
//! Parsing implementation is provided in the [`h1`] and [`h2`] module.
//!
//! [RFC9110]: <https://www.rfc-editor.org/rfc/rfc9110>
//! [`h1`]: crate::h1
//! [`h2`]: crate::h2

mod shared;
mod state;
mod context;
pub mod error;

pub use shared::{Reqline, Header, TargetKind};
pub(crate) use state::{HttpState, insert_header, write_response_head};
pub(crate) use context::HttpContext;
