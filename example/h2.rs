use std::io;
use tcio::bytes::Bytes;
use tokio::{net::TcpListener, runtime::Runtime};
use tsue::body::{Incoming, Full};
use tsue::http::request::Request;
use tsue::http::response::Response;
use tsue::server::Http2Server;
use tsue::service::from_fn;

fn main() -> io::Result<()> {
    Runtime::new().unwrap().block_on(async {
        let io = TcpListener::bind("0.0.0.0:3000").await?;

        println!("listening in {}",io.local_addr().unwrap());

        Http2Server::new(from_fn(handle), io).await;

        Ok(())
    })
}

async fn handle(req: Request<Incoming>) -> Response<Full<Bytes>> {
    dbg!(req);
    <_>::default()
}
