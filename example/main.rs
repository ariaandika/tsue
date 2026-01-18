use std::fs::File;
use std::io;
use std::io::Read;
use std::pin::Pin;
use std::task::Poll;
use tcio::bytes::Bytes;
use tcio::bytes::BytesMut;
use tokio::{net::TcpListener, runtime::Runtime};
use tsue::body::Body;
use tsue::body::Frame;
use tsue::body::Incoming;
use tsue::http::request::Request;
use tsue::http::response::{Parts, Response};
use tsue::server::Http1Server;
use tsue::service::from_fn;

fn main() -> io::Result<()> {
    env_logger::init();
    Runtime::new().unwrap().block_on(async {
        let io = TcpListener::bind("0.0.0.0:3000").await?;

        println!("listening in {}",io.local_addr().unwrap());

        Http1Server::new(io, from_fn(handle)).await;
        Ok(())
    })
}

async fn handle(req: Request<Incoming>) -> Response<Chunked> {
    if req.parts().uri.path() != "/null" {
        // tokio::spawn(async move {
        //     tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        // });
        struct F(tsue::body::Collect);
        impl Future for F {
            type Output = <tsue::body::Collect as Future>::Output;

            fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
                Pin::new(&mut self.0).poll(cx)
            }
        }

        match F(req.into_body().collect()).await {
            Ok(body) => {
                println!("Body len: {}", body.len());
            },
            Err(err) => {
                println!("Body error: {err}");
            },
        };
    }

    Response::from_parts(Parts::default(), Chunked::new())
}

#[allow(unused)]
struct Chunked {
    file: File,
    buffer: [u8; 4 * 1024],
    eof: bool,
}

impl Chunked {
    fn new() -> Self {
        Self {
            file: File::open("Cargo.lock").unwrap(),
            buffer: [0u8; 4 * 1024],
            eof: false,
        }
    }
}

impl Body for Chunked {
    type Data = Bytes;

    type Error = io::Error;

    fn poll_data(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let Chunked { file, buffer, eof } = self.get_mut();
        if *eof {
            return Poll::Ready(None);
        }
        let read = dbg!(file.read(buffer)?);
        if read == 0 {
            *eof = true;
            return Poll::Ready(None);
        }
        Poll::Ready(Some(Ok(Frame::data(
            BytesMut::copy_from_slice(&buffer[..read]).freeze(),
        ))))
    }

    fn is_end_stream(&self) -> bool {
        dbg!(self.eof)
    }

    fn size_hint(&self) -> (u64, Option<u64>) {
        (0, None)
    }
}
