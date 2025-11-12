//! HTTP/1.1 IO Streaming.
mod shared;
mod buffer;

use shared::{Shared, WantsFlag};

pub use buffer::{IoBuffer, IoHandle};
