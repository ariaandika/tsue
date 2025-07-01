use std::io;
use tokio::{net::TcpListener, runtime::Runtime};
use tsue::{request::Request, service::from_fn};

fn main() -> io::Result<()> {
    Runtime::new()
        .unwrap()
        .block_on(async {
            let io = TcpListener::bind("0.0.0.0:3000").await?;

            tsue::rt::serve(io, from_fn(handle)).await;

            Ok(())
        })
}

async fn handle(_req: Request) {
    
}

