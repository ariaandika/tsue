//! share stream between tasks
use std::io;
use bytes::{Buf, Bytes, BytesMut};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, sync::{mpsc::{self, error::TrySendError}, oneshot}};

use crate::body::ResBody;

enum StreamMessage {
    Read {
        tx: oneshot::Sender<io::Result<(usize,BytesMut)>>,
        buffer: BytesMut,
    },
    ReadExact {
        len: usize,
        tx: oneshot::Sender<io::Result<BytesMut>>,
        buffer: BytesMut,
    },
    Write {
        tx: oneshot::Sender<io::Result<()>>,
        head: Bytes,
        body: ResBody,
    },
}

/// share tcp stream via channel
pub fn new_task(stream: TcpStream) -> StreamHandle {
    let (send,recv) = mpsc::channel::<StreamMessage>(2);
    tokio::spawn(task(stream, recv));
    StreamHandle { send }
}

async fn task(mut stream: TcpStream, mut recv: mpsc::Receiver<StreamMessage>) {
    use StreamMessage::*;

    while let Some(message) = recv.recv().await {
        match message {
            Read { tx, mut buffer } => {
                let _ = match stream.read_buf(&mut buffer).await {
                    Ok(ok) => tx.send(Ok((ok,buffer))),
                    Err(err) => tx.send(Err(err)),
                };
            }
            ReadExact { len, tx, mut buffer } => {
                let _ = match stream.read_exact(&mut buffer[..len]).await {
                    Ok(_) => tx.send(Ok(buffer)),
                    Err(err) => tx.send(Err(err)),
                };
            }
            Write { tx, head, body } => {
                let _ = match stream.write_all_buf(&mut Buf::chain(head, body.as_ref())).await {
                    Ok(()) => tx.send(Ok(())),
                    Err(err) => tx.send(Err(err)),
                };
            }
        }
    }
}

/// clonable handle of tcp stream task
pub struct StreamHandle {
    send: mpsc::Sender<StreamMessage>,
}

macro_rules! send {
    ($self:ident,$variant:ident { $($args:ident,)* }) => {{
        use StreamMessage::*;

        let (tx,rx) = oneshot::channel();
        match $self.send.try_send($variant { tx, $($args,)* }) {
            Ok(()) => rx,
            Err(err) => {
                let msg = format!("stream task error: {}",err);
                let tx = match err {
                    TrySendError::Full($variant { tx, .. }) => tx,
                    TrySendError::Closed($variant { tx, .. }) => tx,
                    _ => unreachable!(),
                };
                tx.send(Err(io::Error::new(io::ErrorKind::Other, msg)));
                rx
            },
        }
    }};
}

impl StreamHandle {
    pub fn read(&self, buffer: BytesMut) -> oneshot::Receiver<io::Result<(usize, BytesMut)>> {
        send!(self,Read { buffer, })
    }

    pub fn read_exact(&self, len: usize, buffer: BytesMut) -> oneshot::Receiver<io::Result<BytesMut>> {
        send!(self,ReadExact { len, buffer, })
    }

    pub fn write(&self, head: Bytes, body: ResBody) -> oneshot::Receiver<io::Result<()>> {
        send!(self,Write { head, body, })
    }
}

