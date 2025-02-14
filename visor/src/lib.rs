use http::StatusCode;
use httparse::EMPTY_HEADER;
use std::{
    any::Any,
    future::Future,
    io::{self, IoSlice, Write as _},
    net::{SocketAddr, TcpListener},
    pin::Pin,
    sync::Arc,
    task::Poll,
    time::SystemTime,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    runtime::Builder as Tokio,
};

pub use body::Incoming;

const ADDR: &str = "0.0.0.0:3000";
const HEADER_COUNT: usize = 48;
const RES_STATUS_SIZE: usize = 20;
const BUF_SIZE: usize = 1024;

pub struct Store {
    pub state: Arc<dyn Any + Send + Sync>,
    pub status: &'static mut StatusCode,
    pub method: &'static str,
    pub path: &'static str,
    pub headers: &'static [httparse::Header<'static>],
    pub body: Incoming,
    pub res_header_buf: &'static mut Vec<u8>,
    pub res_body_buf: &'static mut Vec<u8>,
}

/// state is `Arc` internally
pub fn run<S,F,Fut>(state: S, handle: F) -> Result<(), Box<dyn std::error::Error>>
where
    S: Send + Sync + Any + 'static,
    F: Copy + Fn(Store) -> Fut + Send + 'static,
    Fut: Future + Send + 'static,
{
    let tcp = TcpListener::bind(ADDR)?;
    tcp.set_nonblocking(true)?;

    Tokio::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let tcp = tokio::net::TcpListener::from_std(tcp)?;
            let state = Arc::new(state);
            loop {
                match tcp.accept().await {
                    Ok((stream,addr)) => {
                        tokio::spawn(connection::<_, _, Fut>(state.clone(), handle, stream, addr));
                    },
                    Err(err) => {
                        tracing::debug!("failed to accept new connection: {err}");
                    },
                }
            }
        })
}

async fn connection<S,F,Fut>(state: Arc<S>, handle: F, mut stream: TcpStream, _addr: SocketAddr)
where
    S: Send + Sync + Any + 'static,
    F: Copy + Fn(Store) -> Fut + Send + 'static,
    Fut: Future + Send + 'static,
{
    let mut req_buf = Vec::with_capacity(BUF_SIZE);
    let mut headers = [EMPTY_HEADER;HEADER_COUNT];

    let mut res_status_buf = Vec::<u8>::with_capacity(RES_STATUS_SIZE);
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
            match request.parse(req_buf) {
                Ok(httparse::Status::Partial) => continue,
                Ok(httparse::Status::Complete(end)) => end,
                Err(err) => break Err(err.into()),
            }
        };

        let method_ref = {
            let method = request.method.expect("parse always complete");
            (method.as_ptr(),method.len())
        };

        let path_ref = {
            let path = request.path.expect("parse always complete");
            (path.as_ptr(),path.len())
        };

        let mut status = StatusCode::OK;

        res_header_buf.extend_from_slice(b"Date: ");
        write!(&mut res_header_buf, "{}", httpdate::HttpDate::from(SystemTime::now())).ok();
        res_header_buf.extend_from_slice(b"\r\n");

        // body manager
        let body = Incoming::new(body_offset, &mut stream, &mut req_buf, request.headers);

        use std::str::from_utf8_unchecked as b2s;
        use std::slice::from_raw_parts as p2b;

        // call handler
        let store = Store {
            state: Arc::clone(&state) as _,
            status: unsafe { &mut *{ &mut status as *mut StatusCode } },
            method: unsafe { b2s(p2b(method_ref.0, method_ref.1)) },
            path: unsafe { b2s(p2b(path_ref.0, path_ref.1)) },
            headers: unsafe { &*{ request.headers as *mut [httparse::Header] } },
            body,
            res_header_buf: unsafe { &mut *{ &mut res_header_buf as *mut Vec<u8> } },
            res_body_buf: unsafe { &mut *{ &mut res_body_buf as *mut Vec<u8> } },
        };
        handle(store).await;

        res_status_buf.extend_from_slice(b"HTTP/1.1 ");
        res_status_buf.extend_from_slice(status.as_str().as_bytes());
        res_status_buf.push(b' ');
        res_status_buf.extend_from_slice(status.canonical_reason().expect("no canonical reason").as_bytes());
        res_status_buf.extend_from_slice(b"\r\n");

        res_header_buf.extend_from_slice(b"Content-Length: ");
        res_header_buf.extend_from_slice(itoa::Buffer::new().format(res_body_buf.len()).as_bytes());
        res_header_buf.extend_from_slice(b"\r\n\r\n");

        // flush buffer
        // [status, header, body]
        let vectored = [
            IoSlice::new(&res_status_buf),
            IoSlice::new(&res_header_buf),
            IoSlice::new(&res_body_buf)
        ];
        if let Err(err) = stream.write_vectored(&vectored).await {
            break Err(err.into());
        }
        // request complete, clear buffer for subsequent new request
        req_buf.clear();
        res_status_buf.clear();
        res_header_buf.clear();
        res_body_buf.clear();
    };

    if let Err(err) = result {
        tracing::error!("{err}");
    }
}

mod body {
    use super::*;

    pub struct Incoming {
        body_offset: usize,
        stream: &'static mut TcpStream,
        req_ptr: usize,
        req_buf: &'static mut Vec<u8>,
        headers: &'static [httparse::Header<'static>],
        inner: IncomingState,
    }

    #[derive(Default)]
    enum IncomingState {
        Setup,
        Read { expected_len: usize },
        #[default]
        End,
    }

    impl Incoming {
        pub fn new(
            body_offset: usize,
            stream: &mut TcpStream,
            req_buf: &mut Vec<u8>,
            headers: &mut [httparse::Header<'static>],
        ) -> Self {
            Self {
                body_offset,
                stream: unsafe { &mut *{ stream as *mut TcpStream } },
                req_ptr: req_buf as *mut Vec<u8> as usize,
                req_buf: unsafe { &mut *{ req_buf as *mut Vec<u8> } },
                headers: unsafe {
                    std::slice::from_raw_parts(
                        headers.as_ptr(),
                        headers.len(),
                    )
                },
                inner: IncomingState::Setup,
            }
        }
    }

    impl Future for Incoming {
        type Output = io::Result<&'static [u8]>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
            loop {
                match std::mem::take(&mut self.inner) {
                    IncomingState::Setup => {
                        use std::str::from_utf8 as parse_str;

                        let expected_len = match self.headers.iter()
                            .find(|&e|e.name.eq_ignore_ascii_case("content-length"))
                            .and_then(|e|parse_str(e.value).ok()?.parse::<usize>().ok())
                        {
                            Some(0) => return Poll::Ready(Ok(b"")),
                            Some(some) => some,
                            None => {
                                let msg = "failed to parse content length";
                                let err = io::Error::new(io::ErrorKind::InvalidData, msg);
                                return Poll::Ready(Err(err));
                            }
                        };

                        self.inner = IncomingState::Read { expected_len, };
                        continue;
                    }
                    IncomingState::Read { expected_len } => {
                        let buf = unsafe { &mut *{ self.req_ptr as *mut Vec<u8> } };

                        while (self.req_buf.len() - self.body_offset) < expected_len {
                            let mut g = self.stream.read_buf(buf);
                            let pin = std::pin::pin!(g);
                            match pin.poll(cx) {
                                Poll::Ready(Ok(0)) => return Poll::Ready(Err(io::ErrorKind::UnexpectedEof.into())),
                                Poll::Ready(Ok(_)) => {}
                                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                                Poll::Pending => {
                                    self.inner = IncomingState::Read { expected_len };
                                    return Poll::Pending;
                                },
                            }
                        }
                        return Poll::Ready(Ok(&buf[self.body_offset..buf.len()]));
                    }
                    IncomingState::End => unreachable!("poll should not be called after Poll::Ready"),
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

