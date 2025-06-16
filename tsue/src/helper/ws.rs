use base64ct::{Base64, Encoding};
use http::{
    HeaderMap, HeaderValue, StatusCode,
    header::{CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE},
};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use sha1::{Digest, Sha1};
use std::{
    fmt,
    future::{Ready, ready},
};

use crate::{
    body::Body,
    helper::WsUpgrade,
    request::{FromRequest, Request},
    response::{IntoResponse, Response},
};

macro_rules! assert_hdr {
    ($h:ident,$id:ident,$target:literal,$err:literal) => {
        match $h.get($id) {
            Some(header) => if header != &$target[..] {
                return ready(Err(WsUpgradeError::Header($err)))
            },
            None => return ready(Err(WsUpgradeError::Header($err)))
        }
    };
}

// https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API/Writing_WebSocket_servers

impl FromRequest for WsUpgrade {
    type Error = WsUpgradeError;
    type Future = Ready<Result<Self,Self::Error>>;

    fn from_request(req: Request) -> Self::Future {
        let headers = req.headers();
        assert_hdr!(headers, CONNECTION, b"Upgrade", "not an connection upgrade");
        assert_hdr!(headers, UPGRADE, b"websocket", "not an websocket upgrade");
        assert_hdr!(headers, SEC_WEBSOCKET_VERSION, b"13", "unsupported websocket version");
        ready(Ok(Self { req }))
    }
}

impl WsUpgrade {
    pub fn upgrade<F, U>(self, handle: F) -> Response
    where
        F: FnOnce(WebSocket) -> U + Send + 'static,
        U: Future + Send,
    {
        let headers = self.req.headers();
        let key = headers.get(SEC_WEBSOCKET_KEY);
        let derived = HeaderValue::from_bytes(&derive_accept(key.unwrap().as_bytes())).unwrap();

        tokio::spawn(async move {
            let mut req = self.req;
            match hyper::upgrade::on(&mut req).await {
                Ok(io) => {
                    handle(WebSocket { io: TokioIo::new(io) }).await;
                },
                Err(err) => {
                    #[cfg(feature = "log")]
                    log::error!("failed to upgrade websocket: {err}");
                }
            }
        });

        static DEFAULT_HEADERS: std::sync::LazyLock<HeaderMap> = std::sync::LazyLock::new(||{
            const UPGRADE_RES: HeaderValue = HeaderValue::from_static("Upgrade");
            const WEBSOCKET_RES: HeaderValue = HeaderValue::from_static("websocket");
            const KEY_RES: HeaderValue = HeaderValue::from_static("default");

            let mut headers = HeaderMap::new();
            headers.append(CONNECTION, UPGRADE_RES);
            headers.append(UPGRADE, WEBSOCKET_RES);
            headers.append(SEC_WEBSOCKET_ACCEPT, KEY_RES);
            headers
        });

        let mut res = Response::new(Body::default());
        *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
        *res.headers_mut() = DEFAULT_HEADERS.clone();
        res.headers_mut().insert(SEC_WEBSOCKET_ACCEPT, derived);
        res
    }
}

fn derive_accept(key: &[u8]) -> Vec<u8> {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    let mut sha1 = Sha1::default();
    sha1.update(key);
    sha1.update(WS_GUID);
    let key = sha1.finalize();

    let len = Base64::encoded_len(&key);
    let mut dst = vec![0u8; len];
    let res_len = Base64::encode(&key, &mut dst).unwrap().len();
    if dst.len() > res_len {
        dst.truncate(dst.len() - res_len);
    }

    dst
}

// ===== WebSocket =====

#[derive(Debug)]
pub struct WebSocket {
    io: TokioIo<Upgraded>,
}

impl WebSocket {
    pub fn read(&self) {
        let _ = &self.io;
        todo!()
    }
}

// ===== Error =====

/// An Error which can occur during http upgrade.
#[derive(Debug)]
pub enum WsUpgradeError {
    /// Header did not represent http upgrade.
    Header(&'static str),
}

impl std::error::Error for WsUpgradeError { }

impl fmt::Display for WsUpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WsUpgradeError::Header(s) => f.write_str(s),
        }
    }
}

impl IntoResponse for WsUpgradeError {
    fn into_response(self) -> Response {
        StatusCode::BAD_REQUEST.into_response()
    }
}

