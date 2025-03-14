use crate::service::Service;
use log::debug;
use std::{
    io,
    net::{TcpListener as StdListener, ToSocketAddrs},
};
use tokio::net::{TcpListener as TokioListener, TcpStream};

/// listen to tcp listener via tokio runtime
pub fn listen_blocking<S>(addr: impl ToSocketAddrs, service: S) -> io::Result<()>
where
    S: Service<TcpStream> + Clone + Send,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    let tcp = StdListener::bind(addr).map_err(tcp_err)?;
    tcp.set_nonblocking(true)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let tcp = TokioListener::from_std(tcp)?;
            loop {
                match tcp.accept().await {
                    Ok((stream,_)) => { tokio::spawn(service.clone().call(stream)); },
                    Err(err) => { debug!("failed to accept client: {err}"); },
                }
            }
        })
}

fn tcp_err(err: io::Error) -> io::Error {
    io::Error::new(err.kind(), "failed to bind tcp: {err}")
}

