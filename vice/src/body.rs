//! the [`Body`] struct
use bytes::{Bytes, BytesMut};
use std::{io, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

/// http request body
// TODO: do not use arc mutex, Body is hard to construct
pub struct Body {
    content_len: Option<usize>,
    body: BytesMut,
    stream: Arc<Mutex<dyn tokio::io::AsyncRead + Send + Unpin>>,
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Body")
            .field("length", &self.content_len)
            .finish()
    }
}

impl Body {
    pub(crate) fn new(content_len: Option<usize>, body: BytesMut, stream: Arc<Mutex<TcpStream>>) -> Self {
        Self { content_len, body, stream: stream as _ }
    }

    /// return a content length if any
    pub fn content_len(&self) -> Option<usize> {
        self.content_len
    }

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
}


/// http response body
///
/// user typically does not interact with this directly,
/// instead use implementations from [`IntoResponse`]
///
/// [`IntoResponse`]: crate::http::IntoResponse
#[derive(Default)]
pub enum ResBody {
    #[default]
    Empty,
    Static(&'static [u8]),
    Bytes(Bytes),
}

impl ResBody {
    /// return buffer length
    pub fn len(&self) -> usize {
        match self {
            ResBody::Empty => 0,
            ResBody::Static(b) => b.len(),
            ResBody::Bytes(b) => b.len(),
        }
    }

    /// return is buffer length empty
    pub fn is_empty(&self) -> bool {
        match self {
            ResBody::Empty => true,
            ResBody::Static(b) => b.is_empty(),
            ResBody::Bytes(b) => b.is_empty(),
        }
    }

    pub(crate) async fn write(&mut self, stream: &mut TcpStream) -> io::Result<()> {
        match self {
            ResBody::Empty => Ok(()),
            ResBody::Static(b) => stream.write_all(b).await,
            ResBody::Bytes(b) => stream.write_all_buf(b).await,
        }
    }
}

impl From<&'static [u8]> for ResBody {
    fn from(value: &'static [u8]) -> Self {
        Self::Static(value)
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

