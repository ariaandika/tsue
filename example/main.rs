use std::io;
use tcio::{bytes::Bytes, fmt::lossy};
use tokio::{net::TcpListener, runtime::Runtime};
use tsue::{
    body::Incoming,
    http::request::Request,
    http::response::{Parts, Response},
    server::Http1Server,
    service::from_fn,
    body::Full,
};

fn main() -> io::Result<()> {
    Runtime::new().unwrap().block_on(async {
        let io = TcpListener::bind("0.0.0.0:3000").await?;

        println!("listening in {}",io.local_addr().unwrap());

        Http1Server::new(io, from_fn(handle)).await;

        Ok(())
    })
}

async fn handle(req: Request<Incoming>) -> Response<Full<Bytes>> {
    let parts = req.parts();
    dbg!(parts);

    if parts.uri.path() != "/null" {
        let body = req.into_body().collect().await.unwrap();
        println!("{}", lossy(&body.as_slice()));
    }

    Response::from_parts(Parts::default(), Full::new(Bytes::from_static(b"Hello")))
}
