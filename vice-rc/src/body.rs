//! request and response body struct
use bytes::{Bytes, BytesMut};
use std::io;

use crate::stream::{StreamFuture, StreamHandle};

#[derive(Default)]
pub struct Body {
    kind: BodyKind
}

#[derive(Default)]
/// request body
enum BodyKind {
    #[default]
    Empty,
    Chan {
        content_len: usize,
        buffer: BytesMut,
        stream: StreamHandle,
    }
}

impl Body {
    pub(crate) fn empty() -> Body {
        Self { kind: BodyKind::Empty }
    }

    pub(crate) fn new(content_len: usize, buffer: BytesMut, stream: StreamHandle) -> Body {
        Self { kind: BodyKind::Chan { content_len, buffer, stream } }
    }

    /// return content-length if any
    ///
    /// chunked content is not yet supported
    pub fn content_len(&self) -> Option<usize> {
        match self.kind {
            BodyKind::Empty => None,
            BodyKind::Chan { content_len, .. } => Some(content_len),
        }
    }

    /// consume body into [`BytesMut`]
    ///
    /// # Errors
    ///
    /// if content length is missing or invalid, an io error [`io::ErrorKind::InvalidData`] is returned
    ///
    /// otherwise propagate any io error
    pub fn bytes_mut(self) -> StreamFuture<io::Result<BytesMut>> {
        let BodyKind::Chan { stream, buffer, content_len, } = self.kind else {
            return StreamFuture::Empty
        };
        stream.read_exact(content_len, buffer)
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
        f.debug_tuple("Body").field(match &self.kind {
            BodyKind::Empty => b"Empty",
            BodyKind::Chan { .. } => b"Channel",
        }).finish()
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

