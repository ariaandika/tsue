//! the [`Body`] trait
use std::{convert::Infallible, mem, task::Poll};
use bytes::{Buf, Bytes};


/// framed body
pub trait Body {
    /// body frame chunk
    type Output: Buf;
    /// chunk failed error
    type Error;

    /// poll a frame from body
    ///
    /// return none when exhausted
    fn poll_frame(&mut self) -> Poll<Option<Result<Self::Output, Self::Error>>>;

    fn size_hint(&self) -> (usize,Option<usize>) {
        (0, None)
    }
}

impl Body for Bytes {
    type Output = Bytes;

    type Error = Infallible;

    fn poll_frame(&mut self) -> Poll<Option<Result<Self::Output, Self::Error>>> {
        match self.is_empty() {
            true => Poll::Ready(None),
            false => Poll::Ready(Some(Ok(mem::take(&mut *self)))),
        }
    }

    fn size_hint(&self) -> (usize,Option<usize>) {
        (self.len(),Some(self.len()))
    }
}

impl Body for String {
    type Output = Bytes;
    type Error = Infallible;

    fn poll_frame(&mut self) -> Poll<Option<Result<Self::Output, Self::Error>>> {
        match self.is_empty() {
            true => Poll::Ready(None),
            false => Poll::Ready(Some(Ok(mem::take(&mut *self).into_bytes().into()))),
        }
    }

    fn size_hint(&self) -> (usize,Option<usize>) {
        (self.len(),Some(self.len()))
    }
}

impl Body for Vec<u8> {
    type Output = Bytes;
    type Error = Infallible;

    fn poll_frame(&mut self) -> Poll<Option<Result<Self::Output, Self::Error>>> {
        match self.is_empty() {
            true => Poll::Ready(None),
            false => Poll::Ready(Some(Ok(mem::take(&mut *self).into()))),
        }
    }

    fn size_hint(&self) -> (usize,Option<usize>) {
        (self.len(),Some(self.len()))
    }
}

