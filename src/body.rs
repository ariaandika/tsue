//! Request and Response Body.
mod frame;

mod incoming;
mod handle;
mod stream;
mod collect;

mod full;

mod writer;

pub(crate) use writer::BodyWrite;

pub use frame::Frame;
pub use stream::BodyStream;
pub use collect::Collect;
pub use incoming::Incoming;

pub use full::Full;

pub trait Body {
    type Data: tcio::bytes::Buf;

    type Error;

    fn poll_data(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Result<Frame<Self::Data>, Self::Error>>>;

    fn is_end_stream(&self) -> bool;

    fn size_hint(&self) -> (usize, Option<usize>);
}

