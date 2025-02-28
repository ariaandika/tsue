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

// `stream` should not moved when calling service
async fn connection<S>(stream: TcpStream, service: S)
where
    S: Service<Request> + Clone,
    S::Response: IntoResponse,
    S::Error: IntoResponse,
{
    // arc mutex because it will be used when reading body inside service
    let stream = Arc::new(Mutex::new(stream));

    let mut buffer = BytesMut::with_capacity(1024);
    let mut headers = [httparse::EMPTY_HEADER;24];
    let mut res_buffer = BytesMut::with_capacity(1024);

    'root: loop {
        {
            // wait for first request data
            let mut stream = stream.lock().await;
            let _ = match stream.read_buf(&mut buffer).await {
                Ok(0) => break 'root,
                Ok(len) => len,
                Err(err) => { #[cfg(debug_assertions)] tracing::error!("{err}"); break 'root },
            };
        }

        // parse http, rollback to read if partialy parsed
        let mut parse = httparse::Request::new(&mut headers);
        let body_offset = match parse.parse(unsafe {
            // SAFETY: `headers` have the same lifetime with `buffer`
            std::slice::from_raw_parts(buffer.as_ptr(), buffer.len())
        }) {
            Ok(Status::Complete(ok)) => ok,
            Ok(Status::Partial) => continue,
            Err(err) => break tracing::error!("{err}"),
        };

        // split header and body, `buffer` is now empty
        let mut req_buffer = buffer.split();
        let body = req_buffer.split_off(body_offset);
        let header_buffer = req_buffer.freeze();

        let mut content_len = None;
        let mut header_map = HeaderMap::with_capacity(parse.headers.len());

        // collect headers
        for header in parse.headers {
            if header.name.eq_ignore_ascii_case("content-length") {
                content_len = from_utf8(header.value).ok().and_then(|e|e.parse().ok());
            }

            let key = HeaderName::from_bytes(header.name.as_bytes());
            let val = HeaderValue::from_maybe_shared(header_buffer.slice_ref(header.value));

            match (key,val) {
                (Ok(key),Ok(val)) => { header_map.insert(key,val); },
                (Err(err),_) => tracing::warn!("skipping header {}: {err}",header.name),
                (_,Err(err)) => tracing::warn!("skipping header {}: {err}",header.name),
            }
        }

        // all headers now holded by HeaderMap
        drop(header_buffer);

        // construct request
        let req_body = Body::new(content_len, body, stream.clone());
        let mut request = Request::new(req_body);
        let _ = mem::replace(request.headers_mut(), header_map);

        // response scope to make sure all shared bytes dropped
        {
            let response = match service.clone().oneshot(request).await {
                Ok(ok) => ok.into_response(),
                Err(err) => err.into_response(),
            };

            let (mut parts, mut body) = response.into_parts();

            parts.headers.insert(
                HeaderName::from_static("content-length"),
                HeaderValue::from(body.len()),
            );

            res_buffer.extend_from_slice(match parts.version {
                http::Version::HTTP_11 => b"HTTP/1.1 ",
                http::Version::HTTP_2 => b"HTTP/2 ",
                _ => b"HTTP/1.1 ",
            });
            res_buffer.extend_from_slice(parts.status.as_str().as_bytes());
            res_buffer.extend_from_slice(b" ");
            res_buffer.extend_from_slice(
                parts
                .status
                .canonical_reason()
                .unwrap_or("unknown")
                .as_bytes(),
            );
            res_buffer.extend_from_slice(b"\r\n");

            for (key,val) in &parts.headers {
                res_buffer.extend_from_slice(key.as_ref());
                res_buffer.extend_from_slice(b": ");
                res_buffer.extend_from_slice(val.as_ref());
                res_buffer.extend_from_slice(b"\r\n");
            }

            res_buffer.extend_from_slice(b"\r\n");

            let mut stream = stream.lock().await;
            if let Err(err) = stream.write_all_buf(&mut res_buffer).await {
                break tracing::error!("{err}");
            }
            if let Err(err) = body.write(&mut stream).await {
                break tracing::error!("{err}");
            }
        }

        buffer.clear();
        res_buffer.clear();

        // presumably all shared buffer is dropped
        if buffer.try_reclaim(1024) {
            tracing::trace!("buffer reclaimed");
        } else {
            tracing::trace!("unable to reclaim buffer");
            buffer.reserve(1024);
        }
    }
}

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
                    tracing::debug!("{err}");
                }
            }
        }
    }
}

