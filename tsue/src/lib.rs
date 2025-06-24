#![warn(missing_debug_implementations)]
//! http server framework
//!
//! # Quick Start
//!
//! ```ignore
//! use tsue::routing::{Router, get};
//!
//! #[tokio::main]
//! async fn main() -> std::io::Result<()> {
//!     let routes = Router::new()
//!         .route("/", get(index).post(up));
//!
//!     tsue::listen("0.0.0.0:3000", routes).await
//! }
//!
//! async fn index() -> &'static str {
//!     "Tsue Dev!"
//! }
//!
//! async fn up(body: String) {
//!     println!("Client sends: {body}");
//! }
//! ```
pub use tcio::ByteStr;

mod common;
pub mod http;

pub mod body_v2;

pub mod body;
pub mod request;
pub mod response;

pub mod service;
pub mod helper;

pub mod routing;

pub mod rt;

#[cfg(feature = "tokio")]
pub use rt::listen;

#[cfg(feature = "macros")]
pub use tsue_macros::{FromRequest, IntoResponse};
