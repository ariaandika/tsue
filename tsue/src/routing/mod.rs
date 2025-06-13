//! Request routing.

// shared state
mod matcher;

// core routings
mod router;
mod branch;
mod nest;

// utilities
mod state;

// async fn as a Service
mod handler;
mod adapter;

pub(crate) use matcher::Shared;

pub use router::Router;
pub use branch::{Branch, get, post, put, patch, delete};
pub use nest::Nest;
pub use state::State;
pub use adapter::Hyper;

