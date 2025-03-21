//! http server framework
//!
//! # Quick Start
//!
//! ```no_run
//! use std::{io, sync::{atomic::{AtomicU8, Ordering}, Arc}};
//! use vice::{extractor::State, router::{get,Router}};
//! 
//! fn main() -> io::Result<()> {
//!     Router::new()
//!         .route("/", get(index).post(up))
//!         .state(Arc::new(AtomicU8::new(0)))
//!         .listen("0.0.0.0:3000")
//! }
//!
//! async fn index() -> &'static str {
//!     "Vice Dev!"
//! }
//!
//! async fn up(State(counter): State<Arc<AtomicU8>>, body: String) -> String {
//!     format!("{}: {}",counter.fetch_add(1, Ordering::Relaxed),body.to_uppercase())
//! }
//! ```
pub mod request;
pub mod response;

pub mod future;
pub mod service;
pub mod helper;

pub mod route;
pub mod extractor;
pub mod rt;

pub use rt::listen;
