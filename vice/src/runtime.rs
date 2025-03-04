use crate::http::{Request, Response};
use hyper::{server::conn::http1::Builder as Hyper, service::Service};
use hyper_util::rt::TokioIo;
use log::error;
use std::{convert::Infallible, fmt::Display, io, net::ToSocketAddrs};
use tokio::net::TcpListener;

/// entrypoint to run the server
pub fn listen<S>(addr: impl ToSocketAddrs + Display + Clone, service: S) -> io::Result<()>
where
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Future<Output = Result<Response,Infallible>> + Send + 'static,
{
    let tcp = std::net::TcpListener::bind(addr.clone()).map_err(|e|tcp_error(addr, e))?;
    tcp.set_nonblocking(true)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let tcp = TcpListener::from_std(tcp)?;

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
                    Err(err) => {
                        error!("{err}");
                    }
                }
            }
        })
}

fn tcp_error(addr: impl ToSocketAddrs + Display + Clone, err: io::Error) -> io::Error {
    io::Error::new(err.kind(), format!("failed to bind \"{addr}\" :{err}"))
}

