use hyper::body::Incoming;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use std::{convert::Infallible, fmt, io, sync::Arc};
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{
    body::Body,
    common::log,
    request::Request,
    response::Response,
    service::HttpService,
};

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

    let service = Arc::new(Hyper { inner: service });

    loop {
        let service = service.clone();
        match tcp.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(async move {
                    let rt = Builder::new(TokioExecutor::new());
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

/// Service adapter to allow use with [`hyper::service::Service`].
#[derive(Debug)]
pub struct Hyper<S> {
    inner: S,
}

impl<S: HttpService> hyper::service::Service<Request<Incoming>> for Hyper<S> {
    type Response = Response;
    type Error = Infallible;
    type Future = S::Future;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        self.inner.call(req.map(Body::new))
    }
}

impl<S: HttpService> crate::service::Service<Request> for Hyper<S> {
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, req: Request) -> Self::Future {
        self.inner.call(req)
    }
}
