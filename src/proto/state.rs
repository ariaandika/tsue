use tcio::bytes::{Bytes, BytesMut};

use crate::body::{BodyCoder, Codec, error::BodyError};
use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue, standard};
use crate::http::{Extensions, httpdate_now};
use crate::http::{request, response};
use crate::proto::error::{ParseError, ProtoError};
use crate::proto::shared::TargetKind;
use crate::proto::{HttpContext, Reqline};
use crate::uri::{Host, HttpScheme, HttpUri, Path};

#[derive(Debug)]
pub struct HttpState {
    reqline: Reqline,
    headers: HeaderMap,
}

impl HttpState {
    pub fn new(reqline: Reqline, headers: HeaderMap) -> Self {
        Self { reqline, headers }
    }

    pub fn build_context(&self) -> Result<HttpContext, ProtoError> {
        HttpContext::new(&self.reqline, &self.headers)
    }

    pub fn build_decoder(&self) -> Result<BodyCoder, BodyError> {
        BodyCoder::new(&self.headers)
    }

    pub fn build_parts(self) -> Result<request::Parts, ProtoError> {
        let HttpState { reqline, headers } = self;
        let Reqline { method, target, version } = reqline;

        let host = match headers.get(standard::HOST) {
            Some(value) => Bytes::from(value.clone()),
            None => return Err(ProtoError::MissingHost),
        };

        // TODO: infer http scheme ?
        let scheme = HttpScheme::HTTP;
        let kind = TargetKind::new(&method, &target);
        let uri_host;
        let path;

        match kind {
            TargetKind::Origin => {
                uri_host = Host::from_bytes(host)?;
                path = Path::from_bytes(target)?;
            }
            TargetKind::Absolute => {
                let uri = HttpUri::from_bytes(target)?;
                if uri.host().as_bytes() == host.as_slice() {
                    return Err(ParseError::MissmatchHost.into());
                }
                let (_, h, p) = uri.into_parts();
                uri_host = h;
                path = p;
            }
            TargetKind::Asterisk => {
                uri_host = Host::from_bytes(host)?;
                path = Path::from_static(b"*");
            }
            TargetKind::Authority => {
                if target != host {
                    return Err(ParseError::MissmatchHost.into());
                }
                uri_host = Host::from_bytes(target)?;
                path = Path::from_static(b"");
            }
        }
        let uri = HttpUri::from_parts(scheme, uri_host, path);

        Ok(request::Parts {
            method,
            uri,
            version,
            headers,
            extensions: Extensions::new(),
        })
    }
}

pub fn insert_header(
    map: &mut HeaderMap,
    name: BytesMut,
    value: BytesMut,
) -> Result<(), ProtoError> {
    const MAX_HEADERS: usize = 64;

    if map.len() >= MAX_HEADERS {
        return Err(ProtoError::TooManyHeaders);
    }

    let (name, hash) = HeaderName::from_internal(name)?;
    let value = HeaderValue::from_bytes(value)?;
    map.try_append_field(HeaderField::with_hash(name, value, hash))?;

    Ok(())
}

pub fn write_response_head(res: &response::Parts, buf: &mut BytesMut, coding: Option<Codec>) {
    buf.extend_from_slice(res.version.as_str().as_bytes());
    buf.extend_from_slice(b" ");
    buf.extend_from_slice(res.status.as_str().as_bytes());
    buf.extend_from_slice(b"\r\nDate: ");
    buf.extend_from_slice(&httpdate_now()[..]);

    if let Some(coding) = coding {
        match coding {
            Codec::Chunked => {
                // TODO: support compressed transfer-encodings
                buf.extend_from_slice(b"\r\nTransfer-Encoding: chunked\r\n");
            }
            Codec::ContentLength(len) => {
                buf.extend_from_slice(b"\r\nContent-Length: ");
                buf.extend_from_slice(itoa::Buffer::new().format(len).as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
        }
    }

    for f in &res.headers {
        buf.extend_from_slice(f.name().as_str().as_bytes());
        buf.extend_from_slice(b": ");
        buf.extend_from_slice(f.value().as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    buf.extend_from_slice(b"\r\n");
}
