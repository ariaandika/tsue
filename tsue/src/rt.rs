//! Entrypoint of the server
use hyper::server::conn::http1::Builder as Hyper;
use hyper_util::rt::TokioIo;
use std::{fmt::Display, io, sync::Arc};
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{routing, service::HttpService};

/// Entrypoint to run the server
pub async fn listen<S>(addr: impl ToSocketAddrs + Display + Clone, service: S) -> io::Result<()>
where
    S: HttpService,
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
                tokio::spawn(
                    Hyper::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .with_upgrades(),
                );
            }
            Err(_err) => {
                #[cfg(feature = "log")]
                log::error!("failed to connect peer: {_err}");
            }
        }
    }
}
