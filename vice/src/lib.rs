//! http server framework
//!
//! # Quick Start
//!
//! ```no_run
//! use vice::router::{Router, get};
//!
//! fn main() -> std::io::Result<()> {
//!     Router::new()
//!         .route("/", get(index).post(post))
//!         .listen("0.0.0.0:3000")
//! }
//!
//! async fn index() -> &'static str {
//!     "Vice Dev"
//! }
//!
//! async fn post(body: String) -> String {
//!     body.to_lowercase()
//! }
//! ```
pub mod http;
pub mod router;
pub mod util;
pub mod runtime;

#[doc(inline)]
pub use runtime::{listen, HttpService};
