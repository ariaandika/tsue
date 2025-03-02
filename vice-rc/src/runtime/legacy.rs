//! entry point of the server
use crate::http::{Request, IntoResponse};
use crate::body::Body;
use bytes::BytesMut;
use http::{HeaderMap, HeaderName, HeaderValue};
use httparse::Status;
use tokio::io::AsyncWriteExt;
use std::{
    io, mem,
    net::{TcpListener as TcpStd, ToSocketAddrs},
    str::from_utf8,
    sync::Arc,
};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, ToSocketAddrs as TokioAddrs},
    sync::Mutex,
};
use tower::{Service, ServiceExt};

macro_rules! api {
    ($(#[$outer:meta])* async $i1:ident $($tt:tt)*) => {
        api!(@ $(#[$outer])* async fn $i1 $($tt)*);
    };
    ($(#[$outer:meta])* $i1:ident $($tt:tt)*) => {
        api!(@ $(#[$outer])* fn $i1 $($tt)*);
    };
    (@ $(#[$outer:meta])* $i1:ident $i2:ident $($i3:ident)? ($($a1:pat => $t1:ty),*) $body:expr) => {
        $(#[$outer])*
        #[inline]
        pub $i1 $i2 $($i3)? <S>($($a1:$t1),*) -> Result<(), SetupError>
        where
            S: Service<Request> + Clone + Send + 'static,
            S::Response: IntoResponse,
            S::Error: IntoResponse,
            S::Future: Send + 'static,
        {
            $body
        }
    };
}

// keep above macro and `connection` close so that constraint can be
// kept in sync

#[derive(thiserror::Error, Debug)]
pub enum SetupError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to bind tcp: {0}")]
    Tcp(io::Error),
}

api! {
    /// create tokio runtime, tcp listener and handle with service
    listen_blocking(addr => impl ToSocketAddrs, service => S) {
        let tcp = TcpStd::bind(addr).map_err(SetupError::Tcp)?;
        tcp.set_nonblocking(true)?;
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(serve_std(
                tcp,
                service,
            ))
    }
}

api! {
    /// create e tcp listener and handle with service
    async listen(addr => impl TokioAddrs, service => S) {
        serve(TcpListener::bind(addr).await.map_err(SetupError::Tcp)?, service).await
    }
}

api! {
    /// listen to provided std tcp listener and handle with service
    async serve_std(tcp => TcpStd, service => S) {
        serve(TcpListener::from_std(tcp).map_err(SetupError::Tcp)?, service).await
    }
}

api! {
    /// listen to provided tokio tcp listener and handle with service
    async serve(tcp => TcpListener, service => S) {
        loop {
            let service = service.clone();
            match tcp.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(connection(stream, service));
                }
                Err(err) => {
                    log::debug!("{err}");
                }
            }
        }
    }
}

