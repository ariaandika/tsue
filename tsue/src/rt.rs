//! Entrypoint of the server
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as Hyper,
};
use std::{fmt::Display, io, sync::Arc};
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{
    routing::{self, Router},
    service::HttpService,
};

impl<S> Router<S> {
    pub fn listen(self, addr: impl ToSocketAddrs + Display + Clone) -> impl Future<Output = io::Result<()>>
    where
        S: HttpService,
        S::Error: std::error::Error + Send + Sync + 'static,
    {
        listen(addr, self)
    }
}

/// Entrypoint to run the server
pub async fn listen<S>(addr: impl ToSocketAddrs + Display + Clone, service: S) -> io::Result<()>
where
    S: HttpService,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    let tcp = match TcpListener::bind(addr.clone()).await {
        Ok(ok) => ok,
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("failed to bind \"{addr}\" :{err}"),
            ));
        }
    };

    let service = Arc::new(routing::Hyper::new(service));

    loop {
        let service = service.clone();
        match tcp.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(async move {
                    let rt = Hyper::new(TokioExecutor::new());
                    if let Err(_err) = rt.serve_connection_with_upgrades(TokioIo::new(stream), service).await {
                        #[cfg(feature = "log")]
                        log::error!("{_err}");
                    }
                });
            }
            Err(_err) => {
                #[cfg(feature = "log")]
                log::error!("failed to connect peer: {_err}");
            }
        }
    }
}
