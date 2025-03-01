//! request and response body struct
use bytes::{Bytes, BytesMut};
use std::io;
use tokio::sync::oneshot;

#[derive(Default)]
pub struct Body {
    kind: BodyKind
}

#[derive(Default)]
/// request body
pub enum BodyKind {
    #[default]
    Empty,
    Chan {
        content_len: usize,
        tx: oneshot::Sender<()>,
        recv: oneshot::Receiver<io::Result<BytesMut>>,
    }
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            BodyKind::Empty => f.debug_tuple("Body").field(b"Empty").finish(),
            BodyKind::Chan { .. } => f.debug_tuple("Body").finish_non_exhaustive(),
        }
    }
}

impl Body {
    pub(crate) fn empty() -> Body {
        Self { kind: BodyKind::Empty }
    }

    pub(crate) fn new(content_len: usize) -> Body {
        let (tx,_rx) = oneshot::channel::<()>();
        let (_send,recv) = oneshot::channel::<io::Result<BytesMut>>();
        // TODO: spawnd task to read body
        Self { kind: BodyKind::Chan { content_len, tx, recv } }
    }

    pub(crate) fn from_content_len(content_len: Option<usize>) -> Body {
        match content_len {
            Some(len) => Body::new(len),
            None => Body::empty(),
        }
    }

    /*
    /// consume body into [`BytesMut`]
    ///
    /// # Errors
    ///
    /// if content length is missing or invalid, an io error [`io::ErrorKind::InvalidData`] is returned
    ///
    /// otherwise propagate any io error
    pub async fn bytes_mut(self) -> io::Result<BytesMut> {
        let Body { content_len, mut body, stream, } = self;

        let Some(expected_len) = content_len else {
            const MSG: &str = "attempt to read body without content-length";
            return Err(io::Error::new(io::ErrorKind::InvalidData, MSG));
        };

        let read_len = body.len();
        if read_len >= expected_len {
            return Ok(body);
        }

        body.resize(expected_len, 0);

        let mut stream = match stream.try_lock() {
            Ok(ok) => ok,
            Err(err) => {
                let msg = format!("body stream should only have one lock: {err}");
                return Err(io::Error::new(io::ErrorKind::Deadlock, msg));
            }
        };

        stream.read_exact(&mut body[read_len..]).await?;
        Ok(body)
    }

    /// consume body into [`Bytes`]
    ///
    /// this is utility function that propagate [`Body::bytes_mut`]
    pub async fn bytes(self) -> io::Result<Bytes> {
        Ok(self.bytes_mut().await?.freeze())
    }
    */
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

    /*
    pub(crate) async fn write(&mut self, stream: &mut tokio::net::TcpStream) -> io::Result<()> {
        match self {
            ResBody::Empty => Ok(()),
            ResBody::Bytes(b) => tokio::io::AsyncWriteExt::write_all_buf(stream, b).await,
        }
    }
    */
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

