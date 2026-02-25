//! HTTP Body Message.
//!
//! ## Core
//!
//! - [`Body`] the trait that represent a message body
//! - [`Frame`] a single frame of a message body
//!
//! ## Implementation
//!
//! - [`Incoming`] streamed or buffered body
//! - [`Full`] single chunk body
//!

// === impl Body ===
mod full;

// === IO ===
pub(crate) mod shared;
mod handle;
mod incoming;

// === Types ===
mod collect;
pub mod error;


pub use full::Full;
pub use incoming::Incoming;
pub use collect::Collect;

pub trait Body {
    type Data: tcio::bytes::Buf;

    type Error;

    fn poll_data(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Result<Self::Data, Self::Error>>>;

    fn is_end_stream(&self) -> bool;

    fn size_hint(&self) -> (u64, Option<u64>);
}
