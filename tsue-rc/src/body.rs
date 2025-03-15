//! request and response body struct
use bytes::{Bytes, BytesMut};
use std::io;
use crate::task::{StreamFuture, StreamHandle};

/// http request body
//
// this is public api, which user interact to
#[derive(Default)]
pub struct Body {
    chan: Option<BodyChan>
}

struct BodyChan {
    content_len: usize,
    buffer: BytesMut,
    stream: StreamHandle,
}

impl Body {
    pub fn empty() -> Body {
        Self { chan: None }
    }

    pub fn new(content_len: usize, buffer: BytesMut, stream: StreamHandle) -> Body {
        Self { chan: Some(BodyChan { content_len, buffer, stream }) }
    }

    /// return content-length if any
    ///
    /// chunked content is not yet supported
    pub fn content_len(&self) -> Option<usize> {
        self.chan.as_ref().map(|e|e.content_len)
    }

    /// consume body into [`BytesMut`]
    pub fn bytes_mut(self) -> StreamFuture<BytesMut> {
        let Some(BodyChan { stream, buffer, content_len, }) = self.chan else {
            // should if content length is missing or invalid,
            // an io error [`io::ErrorKind::InvalidData`] is returned ?
            return StreamFuture::exact(BytesMut::new())
        };
        let read = buffer.len();
        let read_left = content_len.saturating_sub(read);
        if read_left == 0 {
            return StreamFuture::exact(buffer)
        }
        stream.read_exact(read, read_left, buffer)
    }

    /// consume body into [`Bytes`]
    ///
    /// this is utility function that propagate [`Body::bytes_mut`]
    pub async fn bytes(self) -> io::Result<Bytes> {
        Ok(self.bytes_mut().await?.freeze())
    }
}


#[derive(Default)]
pub enum ResBody {
    #[default]
    Empty,
    Bytes(Bytes),
}

impl ResBody {
    /// return buffer length
    pub fn len(&self) -> usize {
        match self {
            ResBody::Empty => 0,
            ResBody::Bytes(b) => b.len(),
        }
    }

    /// return is buffer length empty
    pub fn is_empty(&self) -> bool {
        match self {
            ResBody::Empty => true,
            ResBody::Bytes(b) => b.is_empty(),
        }
    }
}

impl AsRef<[u8]> for ResBody {
    fn as_ref(&self) -> &[u8] {
        match self {
            ResBody::Empty => &[],
            ResBody::Bytes(bytes) => bytes.as_ref(),
        }
    }
}

impl From<&'static [u8]> for ResBody {
    fn from(value: &'static [u8]) -> Self {
        Self::Bytes(Bytes::from_static(value))
    }
}

impl From<Bytes> for ResBody {
    fn from(value: Bytes) -> Self {
        Self::Bytes(value)
    }
}

impl From<Vec<u8>> for ResBody {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value.into())
    }
}

impl From<String> for ResBody {
    fn from(value: String) -> Self {
        Self::from(value.into_bytes())
    }
}



impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.content_len() {
            Some(len) => f.debug_tuple("Body").field(&len).finish(),
            None => f.debug_tuple("Body").field(&"Empty").finish(),
        }
    }
}

impl std::fmt::Debug for ResBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.debug_tuple("ResBody").field(&"Empty").finish(),
            Self::Bytes(b) => f.debug_tuple("ResBody").field(b).finish(),
        }
    }
}

