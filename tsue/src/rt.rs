//! Entrypoint of the server
#[cfg(feature = "tokio")]
use crate::service::HttpService;

use crate::routing::Router;

impl<S> Router<S> {
    /// Entrypoint to run the server
    #[cfg(feature = "tokio")]
    pub fn listen(
        self,
        addr: impl tokio::net::ToSocketAddrs + std::fmt::Display + Clone,
    ) -> impl Future<Output = std::io::Result<()>>
    where
        S: HttpService,
    {
        listen(addr, self)
    }
}

/// Entrypoint to run the server
#[cfg(feature = "tokio")]
pub async fn listen<S: HttpService>(
    addr: impl tokio::net::ToSocketAddrs + std::fmt::Display + Clone,
    service: S,
) -> std::io::Result<()> {
    use hyper_util::{
        rt::{TokioExecutor, TokioIo},
        server::conn::auto::Builder as Hyper,
    };
    use std::{io, sync::Arc};
    use tokio::net::TcpListener;

    use crate::routing;

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
                    if let Err(_err) = rt
                        .serve_connection_with_upgrades(TokioIo::new(stream), service)
                        .await
                    {
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
