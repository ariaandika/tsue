use base64ct::{Base64, Encoding};
use http::{
    HeaderValue, StatusCode,
    header::{CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE},
};
use hyper::{service::Service, upgrade::Upgraded};
use sha1::{Digest, Sha1};
use std::{
    convert::Infallible,
    future::{Ready, ready},
};

use crate::{
    body::Body,
    helper::WSUpgrade,
    request::Request,
    response::{IntoResponse, Response},
};

macro_rules! get_and_eq {
    ($h:ident,$id:ident,$target:literal) => {
        $h.get($id).map(|e|e == &$target[..]).unwrap_or(false)
    };
}

impl<H> Service<Request> for WSUpgrade<H>
where
    H: WSHandler + Send + Sync + 'static
{
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response,Self::Error>>;

    fn call(&self, mut req: Request) -> Self::Future {
        // https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API/Writing_WebSocket_servers

        let headers = req.headers();
        let conn = get_and_eq!(headers, CONNECTION, b"Upgrade");
        let upgrade = get_and_eq!(headers, UPGRADE, b"websocket");
        let wsver = get_and_eq!(headers, SEC_WEBSOCKET_VERSION, b"13");

        let key = headers.get(SEC_WEBSOCKET_KEY);

        if conn && upgrade && wsver && key.is_some() {
            return ready(Ok(StatusCode::BAD_REQUEST.into_response()));
        }

        let derived = HeaderValue::from_bytes(&derive_accept(key.unwrap().as_bytes())).unwrap();

        tokio::spawn(async move {
            match hyper::upgrade::on(&mut req).await {
                Ok(upgraded) => {
                    H::upgraded(upgraded, req).await;
                },
                Err(err) => {
                    #[cfg(feature = "log")]
                    log::error!("failed to upgrade websocket {err}");
                },
            }
        });

        const UPGRADE_RES: HeaderValue = HeaderValue::from_static("Upgrade");
        const WEBSOCKET_RES: HeaderValue = HeaderValue::from_static("websocket");

        let mut res = Response::new(Body::default());
        *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
        res.headers_mut().append(CONNECTION, UPGRADE_RES);
        res.headers_mut().append(UPGRADE, WEBSOCKET_RES);
        res.headers_mut().append(SEC_WEBSOCKET_ACCEPT, derived);

        ready(Ok(res))
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

// ===== Post Handshake =====

pub trait WSHandler {
    fn upgraded(upgraded: Upgraded, req: Request) -> impl Future + Send;
}

