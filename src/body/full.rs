use std::{convert::Infallible, task::Poll};
use tcio::bytes::Buf;

use crate::body::Frame;
use crate::body::Body;

/// A [`Body`] implementation that consist of a single chunk.
#[derive(Clone, Copy, Debug)]
pub struct Full<D> {
    data: Option<D>,
}

impl<D> Full<D>
where
    D: Buf
{
    /// Creates a new [`Full`].
    #[inline]
    pub fn new(body: D) -> Self {
        Self { data: body.has_remaining().then_some(body) }
    }
}

impl<D> Body for Full<D>
where
    D: Buf
{
    type Data = D;

    type Error = Infallible;

    fn poll_data(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        // SAFETY: self is pinned
        // no `Drop`, nor manual `Unpin` implementation.
        let data = unsafe {
            let me = self.get_unchecked_mut();
            &mut me.data
        };
        Poll::Ready(data.take().map(|d|Ok(Frame::data(d))))
    }

    fn is_end_stream(&self) -> bool {
        self.data.is_none()
    }

    fn size_hint(&self) -> (u64, Option<u64>) {
        match &self.data {
            Some(d) => {
                let remain = d.remaining() as u64;
                (remain, Some(remain))
            },
            None => (0, None),
        }
    }
}

impl<D> Default for Full<D>
where
    D: Buf
{
    #[inline]
    fn default() -> Self {
        Self { data: None }
    }
}
