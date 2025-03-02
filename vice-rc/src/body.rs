//! request and response body struct
use bytes::{Bytes, BytesMut};
use std::io;
use tokio::{net::TcpStream, sync::oneshot};

#[derive(Default)]
pub struct Body {
    kind: BodyKind
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Body").field(match &self.kind {
            BodyKind::Empty => b"Empty",
            BodyKind::Chan { .. } => b"Channel",
        }).finish()
    }
}

#[derive(Default)]
/// request body
enum BodyKind {
    #[default]
    Empty,
    Chan {
        content_len: usize,
        tx: oneshot::Sender<()>,
        recv: oneshot::Receiver<io::Result<BytesMut>>,
    }
}

impl Body {
    pub(crate) fn empty() -> Body {
        Self { kind: BodyKind::Empty }
    }

    pub(crate) fn new(content_len: usize) -> (Body,tokio::task::JoinHandle<TcpStream>) {
        let (tx,rx) = oneshot::channel::<()>();
        let (send,recv) = oneshot::channel::<io::Result<BytesMut>>();
        let body = Self { kind: BodyKind::Chan { content_len, tx, recv } };
        let handle = tokio::spawn(Body::task(content_len,rx,send));
        (body,handle)
    }

    pub(crate) fn from_content_len(content_len: Option<usize>) -> Body {
        todo!()
        // match content_len {
        //     Some(len) => Body::new(len),
        //     None => Body::empty(),
        // }
    }

    pub fn content_len(&self) -> usize {
        match self.kind {
            BodyKind::Empty => 0,
            BodyKind::Chan { content_len, .. } => content_len,
        }
    }

    /// consume body into [`BytesMut`]
    ///
    /// # Errors
    ///
    /// if content length is missing or invalid, an io error [`io::ErrorKind::InvalidData`] is returned
    ///
    /// otherwise propagate any io error
    pub async fn bytes_mut(self) -> io::Result<BytesMut> {
        let BodyKind::Chan { content_len, tx, recv } = self.kind else {
            return Ok(BytesMut::new())
        };
        // let Body { content_len, mut body, stream, } = self;
        /*

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
        */
        todo!()
    }

    /*
    /// consume body into [`Bytes`]
    ///
    /// this is utility function that propagate [`Body::bytes_mut`]
    pub async fn bytes(self) -> io::Result<Bytes> {
        Ok(self.bytes_mut().await?.freeze())
    }
    */
    async fn task(
        content_len: usize,
        rx: oneshot::Receiver<()>,
        send: oneshot::Sender<io::Result<BytesMut>>,
    ) -> TcpStream {
        todo!()
    }
}

pub struct BodyFuture {

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

