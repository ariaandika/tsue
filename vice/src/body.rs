use bytes::{Bytes, BytesMut};
use http_body::Frame;
use std::{
    convert::Infallible,
    io, mem,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

pub struct Body {
    content_len: Option<usize>,
    body: BytesMut,
    stream: Arc<Mutex<TcpStream>>,
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Body")
            .field(&self.content_len)
            .finish()
    }
}

impl Body {
    pub fn new(content_len: Option<usize>, body: BytesMut, stream: Arc<Mutex<TcpStream>>) -> Self {
        Self { content_len, body, stream }
    }

    pub fn content_len(&self) -> Option<usize> {
        self.content_len
    }

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
    pub fn len(&self) -> usize {
        match self {
            ResBody::Empty => 0,
            ResBody::Bytes(b) => b.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ResBody::Empty => true,
            ResBody::Bytes(b) => b.is_empty(),
        }
    }

    pub async fn write(&mut self, stream: &mut TcpStream) -> io::Result<()> {
        match self {
            ResBody::Empty => {},
            ResBody::Bytes(b) => stream.write_all_buf(b).await?,
        }

        Ok(())
    }
}

impl http_body::Body for ResBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if self.is_empty() {
            return Poll::Ready(None);
        }
        match &mut *self {
            ResBody::Empty => Poll::Ready(None),
            ResBody::Bytes(b) => Poll::Ready(Some(Ok(Frame::data(mem::take(b))))),
        }
    }

    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::with_exact(self.len() as u64)
    }

    fn is_end_stream(&self) -> bool {
        self.is_empty()
    }
}

