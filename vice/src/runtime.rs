use anyhow::Context;
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1::Builder as Hyper,
    service::service_fn,
    Request, Response,
};
use hyper_util::rt::TokioIo;
use std::{convert::Infallible, net::ToSocketAddrs};
use tokio::net::TcpListener;


pub fn listen_blocking(addr: impl ToSocketAddrs) -> anyhow::Result<()> {
    let tcp = std::net::TcpListener::bind(addr).context("failed to bind tcp")?;
    tcp.set_nonblocking(true)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let tcp = TcpListener::from_std(tcp)?;

            loop {
                match tcp.accept().await {
                    Ok((stream, _)) => {
                        tokio::spawn(
                            Hyper::new()
                                .serve_connection(TokioIo::new(stream), service_fn(connection))
                                .with_upgrades(),
                        );
                    }
                    Err(err) => {
                        tracing::error!("{err}");
                    }
                }
            }
        })
}

async fn connection(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Default::default())
}

