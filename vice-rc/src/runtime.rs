use std::{io, net::{TcpListener as StdListener, ToSocketAddrs}};
use tokio::net::{TcpListener as TokioListener, TcpStream};

use crate::service::Service;


/// listen to tcp listener via tokio runtime
pub fn listen_blocking<S>(addr: impl ToSocketAddrs, service: S) -> Result<(), SetupError>
where
    S: Service<TcpStream> + Clone + Send,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    let tcp = StdListener::bind(addr).map_err(SetupError::Tcp)?;
    tcp.set_nonblocking(true)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let tcp = TokioListener::from_std(tcp)?;
            loop {
                match tcp.accept().await {
                    Ok((stream,_)) => { tokio::spawn(service.clone().call(stream)); },
                    Err(err) => { tracing::debug!("failed to accept client: {err}"); },
                }
            }
        })
}

#[derive(thiserror::Error, Debug)]
pub enum SetupError {
    #[error("failed to bind tcp: {0}")]
    Tcp(io::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

