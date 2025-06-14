//! Request routing.

// shared state
mod matcher;
mod zip;

// core routings
mod router;
mod fallback;
mod branch;
mod nest;

// async fn
mod handler;

// utilities
mod state;
mod adapter;

// ===== reexports =====

pub(crate) use matcher::Shared;

pub use router::Router;
pub use branch::{Branch, get, post, put, patch, delete};
pub use nest::Nest;
pub use state::State;
pub use adapter::Hyper;

