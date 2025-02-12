use httparse::EMPTY_HEADER;
use std::{
    future::Future,
    io::{self, IoSlice},
    net::{SocketAddr, TcpListener}, pin::Pin, task::Poll,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    runtime::Builder as Tokio, sync::oneshot,
};

pub use body::Body;

const ADDR: &'static str = "0.0.0.0:3000";
const HEADER_COUNT: usize = 48;
const BUF_SIZE: usize = 1024;

pub struct Store {
    pub headers: &'static [httparse::Header<'static>],
    pub body: Body,
    pub res_header_buf: &'static mut Vec<u8>,
    pub res_body_buf: &'static mut Vec<u8>,
}

pub fn run<S,F,Fut>(state: S, handle: F) -> Result<(), Box<dyn std::error::Error>>
where
    S: Clone + Send + 'static,
    F: Copy + Fn(S,Store) -> Fut + Send + 'static,
    Fut: Future + Send + 'static,
{
    let tcp = TcpListener::bind(ADDR)?;
    tcp.set_nonblocking(true)?;

    Tokio::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let tcp = tokio::net::TcpListener::from_std(tcp)?;

            loop {
                match tcp.accept().await {
                    Ok((stream,addr)) => {
                        tokio::spawn(connection::<_, _, Fut>(
                            state.clone(), handle, stream, addr
                        ));
                    },
                    Err(err) => {
                        tracing::debug!("failed to accept new connection: {err}");
                    },
                }
            }
        })
}

async fn connection<S,F,Fut>(state: S, handle: F, mut stream: TcpStream, _addr: SocketAddr)
where
    S: Clone + Send + 'static,
    F: Copy + Fn(S,Store) -> Fut + Send + 'static,
    Fut: Future + Send + 'static,
{
    let mut req_buf = Vec::with_capacity(BUF_SIZE);
    let mut headers = [EMPTY_HEADER;HEADER_COUNT];

    let mut res_header_buf = Vec::<u8>::with_capacity(BUF_SIZE / 2);
    let mut res_body_buf = Vec::<u8>::with_capacity(BUF_SIZE);

    let result: Result<(), Box<dyn std::error::Error>> = loop {
        match stream.read_buf(&mut req_buf).await {
            Ok(0) => break Ok(()),
            Ok(_) => {}
            Err(err) => break Err(err.into()),
        }

        let mut request = {
            let headers = unsafe { &mut *{ &mut headers as *mut [httparse::Header<'static>] } };
            httparse::Request::new(headers)
        };

        let body_offset = {
            let req_buf = unsafe { &*{ &req_buf[..] as *const [u8] } };
            match request.parse(&req_buf) {
                Ok(httparse::Status::Partial) => continue,
                Ok(httparse::Status::Complete(end)) => end,
                Err(err) => break Err(err.into()),
            }
        };

        // body manager
        let body = Body::new(body_offset, &mut stream, &mut req_buf, &request.headers);

        // call handler
        let store = Store {
            headers: unsafe { &*{ request.headers as *mut [httparse::Header] } },
            body,
            res_header_buf: unsafe { &mut *{ &mut res_header_buf as *mut Vec<u8> } },
            res_body_buf: unsafe { &mut *{ &mut res_body_buf as *mut Vec<u8> } },
        };
        handle(state.clone(),store).await;

        // flush buffer
        // [header, body]
        let vectored = [IoSlice::new(&res_header_buf), IoSlice::new(&res_body_buf)];
        if let Err(err) = stream.write_vectored(&vectored).await {
            break Err(err.into());
        }

        // request complete, clear buffer for subsequent new request
        req_buf.clear();
        res_header_buf.clear();
        res_body_buf.clear();
    };

    if let Err(err) = result {
        tracing::error!("{err}");
    }
}

mod body {
    use super::*;

    pub struct Body {
        body_offset: usize,
        /// Vec<u8>
        req_ptr: usize,
        inner: BodyState,
    }

    impl Body {
        pub fn new(
            body_offset: usize,
            stream: &mut TcpStream,
            req_buf: &mut Vec<u8>,
            headers: &[httparse::Header<'static>]
        ) -> Self {
            let (send,recv) = oneshot::channel::<()>();
            let (call,back) = oneshot::channel::<io::Result<()>>();
            let req_ptr = req_buf as *mut Vec<u8> as usize;
            tokio::spawn(body::reader(
                body_offset,
                stream as *mut TcpStream as usize,
                req_ptr,
                (headers.as_ptr() as usize, headers.len()), // slice cannot be directly usized
                recv,
                call,
            ));
            Self {
                body_offset,
                req_ptr,
                inner: BodyState::Send { send, back },
            }
        }
    }

    #[derive(Default)]
    enum BodyState {
        Send { send: oneshot::Sender<()>, back: oneshot::Receiver<io::Result<()>> },
        Recv { back: oneshot::Receiver<io::Result<()>> },
        #[default]
        End,
    }

    pub async fn reader(
        body_offset: usize,
        stream_ptr: usize,
        req_ptr: usize,
        headers_ref: (usize,usize),
        recv: oneshot::Receiver<()>,
        call: oneshot::Sender<io::Result<()>>,
    ) {
        let Ok(()) = recv.await else {
            tracing::trace!("Body never read");
            return;
        };

        let stream = unsafe { &mut *{ stream_ptr as *mut TcpStream } };
        let mut req_buf = unsafe { &mut *{ req_ptr as *mut Vec<u8> } };

        let (headers_ptr,headers_len) = headers_ref;
        let headers = unsafe {
            std::slice::from_raw_parts(headers_ptr as *const httparse::Header<'_>, headers_len)
        };

        // Read Body
        use std::str::from_utf8 as parse_str;

        let expected_len = match headers.iter()
            .find(|&e|e.name.eq_ignore_ascii_case("content-length"))
            .and_then(|e|parse_str(e.value).ok()?.parse::<usize>().ok())
        {
            Some(some) => some,
            None => {
                let err = io::Error::new(io::ErrorKind::InvalidData, "failed to parse content length");
                call.send(Err(err)).ok();
                return;
            }
        };

        // keep reading until expected len reached
        while (req_buf.len() - body_offset) < expected_len {
            match stream.read_buf(&mut req_buf).await {
                Ok(0) => {
                    call.send(Err(io::ErrorKind::UnexpectedEof.into())).ok();
                    return;
                }
                Ok(_) => {}
                Err(err) => {
                    call.send(Err(err)).ok();
                    return;
                }
            }
        }

        // send read ok signal
        call.send(Ok(())).ok();
    }

    impl Future for Body {
        type Output = io::Result<&'static [u8]>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
            loop {
                match std::mem::take(&mut self.inner) {
                    // body reader task is not yet polled
                    BodyState::Send { send, back } => {
                        send.send(()).expect("the spawned thread recv never drop before this");
                        self.inner = BodyState::Recv { back };
                        continue;
                    }
                    // wait for body read task
                    BodyState::Recv { mut back } => {
                        let pin = Pin::new(&mut back);
                        match pin.poll(cx) {
                            Poll::Ready(result) => {
                                return match result.expect("the spawned thread call never drop without sending msg") {
                                    Ok(()) => {
                                        let buf: &'static Vec<u8> = unsafe { &mut *{ self.req_ptr as *mut Vec<u8> } };
                                        Poll::Ready(Ok(&buf[self.body_offset..buf.len()]))
                                    },
                                    Err(err) => {
                                        Poll::Ready(Err(err))
                                    },
                                }
                            }
                            Poll::Pending => {
                                self.inner = BodyState::Recv { back };
                                return Poll::Pending;
                            }
                        }
                    }
                    BodyState::End => unreachable!("poll should not be called after Poll::Ready"),
                }
            }
        }
    }
}


pub mod util {
    pub fn display_str(buf: &[u8]) -> &str {
        std::str::from_utf8(buf).unwrap_or("<NON-UTF8>")
    }
}

