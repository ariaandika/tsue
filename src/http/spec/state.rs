use tcio::bytes::{Bytes, BytesMut};

use super::ProtoError;
use super::{HttpContext, BodyDecoder};
use crate::h1::parser::{Reqline};
use crate::headers::standard::{CONTENT_LENGTH, HOST};
use crate::headers::{HeaderMap, HeaderName, HeaderValue};
use crate::http::spec::Coding;
use crate::http::{Extensions, httpdate_now};
use crate::http::{request, response};
use crate::uri::HttpScheme;

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

    pub fn build_decoder(&self) -> Result<BodyDecoder, ProtoError> {
        BodyDecoder::new(&self.headers)
    }

    pub fn build_parts(self) -> Result<request::Parts, ProtoError> {
        let host = match self.headers.get(HOST) {
            Some(ok) => Bytes::from(ok.clone()),
            None => return Err(ProtoError::MissingHost),
        };
        let uri = self.reqline.target.build_origin(host, HttpScheme::HTTP)?;

        Ok(request::Parts {
            method: self.reqline.method,
            uri,
            version: self.reqline.version,
            headers: self.headers,
            extensions: Extensions::new(),
        })
    }
}

pub fn insert_header(
    map: &mut HeaderMap,
    mut name: BytesMut,
    value: BytesMut,
) -> Result<(), ProtoError> {
    const MAX_HEADERS: usize = 64;

    if map.len() >= MAX_HEADERS {
        return Err(ProtoError::TooManyHeaders);
    }

    name.make_ascii_lowercase();
    map.append(
        HeaderName::from_bytes_lowercase(name)?,
        HeaderValue::from_bytes(value)?,
    );

    Ok(())
}

pub fn write_response(res: &response::Parts, buf: &mut BytesMut, coding: &Coding) {
    buf.extend_from_slice(res.version.as_str().as_bytes());
    buf.extend_from_slice(b" ");
    buf.extend_from_slice(res.status.as_str().as_bytes());
    buf.extend_from_slice(b"\r\nDate: ");
    buf.extend_from_slice(&httpdate_now()[..]);

    match coding {
        Coding::Empty => {
            buf.extend_from_slice(b"\r\nContent-Length: 0\r\n");
        }
        Coding::Chunked(_) => {
            // TODO: support compressed transfer-encodings
            buf.extend_from_slice(b"\r\nTransfer-Encoding: chunked\r\n");
        }
        Coding::ContentLength(len) => {
            buf.extend_from_slice(b"\r\nContent-Length: ");
            buf.extend_from_slice(itoa::Buffer::new().format(*len).as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
    }

    for (key, val) in &res.headers {
        buf.extend_from_slice(key.as_str().as_bytes());
        buf.extend_from_slice(b": ");
        buf.extend_from_slice(val.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    buf.extend_from_slice(b"\r\n");
}
