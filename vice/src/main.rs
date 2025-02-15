use std::{io, net::TcpListener as StdTcpListener};

use tokio::net::TcpListener;


const ADDR: &str = "0.0.0.0:3000";

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

                tokio::spawn(async move {
                    let _ = stream;
                });
            }
        })
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("failed to bind to tcp: {0}")]
    Tcp(io::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

