use std::io;
use tcio::fmt::lossy;
use tokio::{net::TcpListener, runtime::Runtime};
use tsue::{
    body::Body,
    request::Request,
    response::{Parts, Response},
    service::from_fn,
};

fn main() -> io::Result<()> {
    Runtime::new()
        .unwrap()
        .block_on(async {
            let io = TcpListener::bind("0.0.0.0:3000").await?;

            tsue::rt::serve(io, from_fn(handle)).await;
            Ok(())
        })
}

async fn handle(req: Request) -> Response {
    let parts = req.parts();
    println!("> {} {} {}", parts.method, parts.uri, parts.version);

    if parts.uri.path() != "/null" {
        let body = req.into_body().collect().await.unwrap();
        println!("{}",lossy(&body));
    }

    Response::from_parts(Parts::default(), Body::new(&b"Hell"[..]))
}

