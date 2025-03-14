//! entrypoint to start the server
use crate::service::{tcp::TcpService, HttpService, Service};
use log::debug;
use std::{
    io,
    net::{TcpListener as StdListener, ToSocketAddrs},
    sync::Arc,
};
use tokio::net::TcpListener as TokioListener;

/// listen to tcp listener via tokio runtime
pub fn listen<S>(addr: impl ToSocketAddrs, service: S) -> io::Result<()>
where
    S: HttpService
{
    let tcp = StdListener::bind(addr).map_err(tcp_err)?;
    tcp.set_nonblocking(true)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let tcp = TokioListener::from_std(tcp)?;
            let arc = Arc::new(service);
            loop {
                match tcp.accept().await {
                    Ok((stream,_)) => { tokio::spawn(TcpService::new(arc.clone()).call(stream)); },
                    Err(err) => { debug!("failed to accept client: {err}"); },
                }
            }
        })
}

fn tcp_err(err: io::Error) -> io::Error {
    io::Error::new(err.kind(), format!("failed to bind tcp: {err}"))
}

