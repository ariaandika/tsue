//! Request routing.

// core routings
mod router;
mod branch;
mod nest;
mod matcher;

// utilities
mod state;
mod adapter;

// async fn as a Service
mod handler;

pub use router::Router;
pub use branch::{Branch, get, post, put, patch, delete};
pub use state::State;
pub use adapter::Hyper;

