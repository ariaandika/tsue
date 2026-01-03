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

// === HTTP ===
mod chunked;
mod coder;

// === IO ===
pub(crate) mod handle;
mod frame;
mod incoming;
mod collect;

pub mod error;


pub use full::Full;
pub use chunked::EncodedBuf;
pub use coder::BodyCoder;
pub use frame::Frame;
pub use incoming::Incoming;
pub use collect::Collect;

use std::pin::Pin;
use std::task::{Poll, Context};
use tcio::bytes::Buf;

#[allow(clippy::type_complexity)]
pub trait Body {
    type Data: Buf;

    type Error;

    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>>;

    fn is_end_stream(&self) -> bool;

    fn size_hint(&self) -> (u64, Option<u64>);
}

/// HTTP Body Codec.
#[derive(Copy, Clone, Debug)]
pub enum Codec {
    Chunked,
    ContentLength(u64),
}
