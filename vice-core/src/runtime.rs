use std::{io, net::{TcpListener as TcpStd, ToSocketAddrs}};
use tokio::net::{TcpListener, TcpStream};
use crate::service::Service;


/// bind tcp, spawn tokio runtime, and serve indefinitely
pub fn listen_block<S>(addr: impl ToSocketAddrs, service: S) -> Result<(), SetupError>
where
    S: Service<TcpStream> + Clone + Send + 'static,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    let tcp = TcpStd::bind(addr).map_err(SetupError::Tcp)?;
    tcp.set_nonblocking(true)?;

    tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()?
    .block_on(async {
        let tcp = TcpListener::from_std(tcp).map_err(SetupError::Tcp)?;
        loop {
            let mut service = service.clone();
            match tcp.accept().await {
                Ok((stream,_)) => {
                    tokio::spawn(service.call(stream));
                },
                Err(err) => {
                    tracing::debug!("{err}");
                },
            }
        }
    })
}

#[derive(thiserror::Error, Debug)]
pub enum SetupError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to bind tcp: {0}")]
    Tcp(io::Error),
}

