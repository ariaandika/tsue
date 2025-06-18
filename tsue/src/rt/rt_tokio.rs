use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as Hyper,
};
use std::{fmt, io, sync::Arc};
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{common::log, routing, service::HttpService};

/// Start server using hyper and tokio.
pub async fn listen<S: HttpService>(
    addr: impl ToSocketAddrs + fmt::Display + Clone,
    service: S,
) -> io::Result<()> {

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
                    if let Err(err) = rt
                        .serve_connection_with_upgrades(TokioIo::new(stream), service)
                        .await
                    {
                        log!("{err}")
                    }
                });
            }
            Err(err) => log!("failed to connect peer: {err}"),
        }
    }
}
