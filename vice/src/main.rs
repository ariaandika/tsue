use std::{io, net::TcpListener as StdTcpListener};

use tokio::net::{TcpListener, TcpStream};
use vice::service::{connection::Connection, service_fn, Service};


const ADDR: &str = "0.0.0.0:3000";

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("failed to bind tcp: {0}")]
    Tcp(io::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

fn main() -> Result<(), Error> {
    let tcp = StdTcpListener::bind(ADDR).map_err(Error::Tcp)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let tcp = TcpListener::from_std(tcp)?;

            loop {
                let (stream, _addr) = match tcp.accept().await {
                    Ok(ok) => ok,
                    Err(err) => {
                        eprintln!("{err}");
                        continue;
                    },
                };

                tokio::spawn(Connection::new(service_fn(handle)).call(stream));
            }
        })
}

async fn handle((stream,buf): (TcpStream, Vec<u8>)) -> Result<(TcpStream, Vec<u8>), Error> {
    dbg!(String::from_utf8(buf)).ok();
    Ok((stream,Vec::from(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")))
}


