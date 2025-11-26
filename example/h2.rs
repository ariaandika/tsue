use std::io;
use tokio::{net::TcpListener, runtime::Runtime};
use tsue::h2::Connection;

fn main() -> io::Result<()> {
    Runtime::new().unwrap().block_on(async {
        let io = TcpListener::bind("0.0.0.0:3000").await?;

        println!("listening in {}",io.local_addr().unwrap());

        let (io, _) = io.accept().await.unwrap();
        Connection::new(io).await;

        Ok(())
    })
}

