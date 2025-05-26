//! Request routing.

// core routings
mod router;
mod branch;
mod nest;
mod matcher;

// utilities
mod state;

// async fn as a Service
mod handler;
mod adapter;

pub use router::Router;
pub use branch::{Branch, get, post, put, patch, delete};
pub use state::State;
pub use adapter::Hyper;

