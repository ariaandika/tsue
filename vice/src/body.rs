use std::{convert::Infallible, task::Poll};

use bytes::{Buf, Bytes};

/// framed body
pub trait Body {
    type Output: Buf;
    type Error;

    fn poll_frame(&mut self) -> Poll<Option<Result<Self::Output, Self::Error>>>;

    fn is_end_stream(&self) -> bool;

    fn size_hint(&self) -> (usize,Option<usize>) {
        (0, None)
    }
}

impl Body for String {
    type Output = Bytes;
    type Error = Infallible;

    fn poll_frame(&mut self) -> Poll<Option<Result<Self::Output, Self::Error>>> {
        match !self.is_empty() {
            true => Poll::Ready(Some(Ok(std::mem::take(&mut *self).into_bytes().into()))),
            false => Poll::Ready(None),
        }
    }

    fn is_end_stream(&self) -> bool {
        self.is_empty()
    }

    fn size_hint(&self) -> (usize,Option<usize>) {
        (self.len(),Some(self.len()))
    }
}

impl Body for Vec<u8> {
    type Output = Bytes;
    type Error = Infallible;

    fn poll_frame(&mut self) -> Poll<Option<Result<Self::Output, Self::Error>>> {
        match !self.is_empty() {
            true => Poll::Ready(Some(Ok(std::mem::take(&mut *self).into()))),
            false => Poll::Ready(None),
        }
    }

    fn is_end_stream(&self) -> bool {
        self.is_empty()
    }

    fn size_hint(&self) -> (usize,Option<usize>) {
        (self.len(),Some(self.len()))
    }
}

