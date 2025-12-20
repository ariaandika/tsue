use tcio::bytes::{Bytes, BytesMut};
use std::{
    io,
    pin::Pin,
    task::Poll,
};

use super::Incoming;

/// A future returned from [`Body::collect`], which buffer entire request body.
#[derive(Debug)]
pub struct Collect {
    body: Incoming,
    buffer: Option<Buffer>,
}

/// This state can optimize in case of only one Bytes returned from Stream, which will prevent
/// copying in concatenation.
#[derive(Debug)]
enum Buffer {
    None,
    Ref(Bytes),
    Mut(BytesMut),
}

impl Collect {
    pub(crate) fn new(body: Incoming) -> Self {
        Self {
            body,
            buffer: Some(Buffer::None),
        }
    }

    fn take_buffer(&mut self) -> Bytes {
        match self.buffer.take().expect("poll after complete") {
            Buffer::None => Bytes::new(),
            Buffer::Ref(bytes) => bytes,
            Buffer::Mut(bytes_mut) => bytes_mut.freeze(),
        }
    }
}

impl Future for Collect {
    type Output = io::Result<Bytes>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        let me = self.get_mut();

        todo!()

        // match &mut me.body.repr() {
        //     Repr::Bytes(b) => Poll::Ready(if b.is_empty() {
        //         Err(io::ErrorKind::QuotaExceeded.into())
        //     } else {
        //         Ok(take(b))
        //     }),
        //     Repr::Handle(handle) => {
        //         while handle.has_remaining() {
        //             let data = ready!(handle.poll_read(cx))?;
        //
        //             match me.buffer.as_mut().expect("poll after complete") {
        //                 Buffer::None => me.buffer = Some(Buffer::Mut(data)),
        //                 Buffer::Mut(bytesm) => {
        //                     // #[cfg(debug_assertions)]
        //                     // let ptr = bytesm.as_ptr();
        //
        //                     bytesm.unsplit(data);
        //
        //                     // `IoHandle` returns bytes that are contiguous,
        //                     // so it should never copy
        //                     // debug_assert_eq!(ptr, bytesm.as_ptr());
        //                 },
        //                 Buffer::Ref(_) => unreachable!("Repr::Handle never use Bytes"),
        //             };
        //         }
        //
        //         Poll::Ready(Ok(me.take_buffer()))
        //     },
        //     Repr::Stream(stream) => {
        //         while stream.has_remaining() {
        //             let data = ready!(stream.poll_read(cx))?;
        //
        //             match me.buffer.as_mut().expect("poll after complete") {
        //                 Buffer::None => me.buffer = Some(Buffer::Ref(data)),
        //                 Buffer::Ref(bytes) => {
        //                     // Stream returns more than one Bytes,
        //                     // concatenation requires copy
        //                     let mut bytesm = BytesMut::with_capacity(bytes.len() + stream.remaining());
        //                     bytesm.extend_from_slice(bytes);
        //                     bytesm.extend_from_slice(&data);
        //                     me.buffer = Some(Buffer::Mut(bytesm));
        //                 },
        //
        //                 Buffer::Mut(bytes_mut) => {
        //                     bytes_mut.extend_from_slice(&data);
        //                 },
        //             };
        //         }
        //
        //         Poll::Ready(Ok(me.take_buffer()))
        //     },
        // }
    }
}
