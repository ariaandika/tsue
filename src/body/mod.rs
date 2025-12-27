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
mod frame;
pub(crate) mod handle;
mod stream;
mod collect;
mod incoming;
mod full;
mod writer;
pub mod error;

pub(crate) use writer::BodyWrite;
pub use frame::Frame;
pub use incoming::Incoming;
pub use full::Full;
pub use stream::BodyStream;
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

