use std::io;
use tcio::bytes::Bytes;
use tokio::{net::TcpListener, runtime::Runtime};
use tsue::body::Full;
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

async fn handle(req: Request<Incoming>) -> Response<Full<Bytes>> {
    if req.parts().uri.path() != "/null" {
        let body = req.into_body().collect().await.unwrap();
        println!("Body len: {}", body.len());
    }

    Response::from_parts(Parts::default(), Full::new(Bytes::from_static(b"Hello")))
}
