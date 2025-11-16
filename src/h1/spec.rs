//! HTTP/1.1 Semantics.
use tcio::bytes::{Bytes, BytesMut};

use super::parser::{Header, Reqline};
use crate::{
    headers::{
        HeaderMap, HeaderName, HeaderValue,
        error::HeaderError,
        standard::{CONTENT_LENGTH, HOST},
    },
    http::{Extensions, httpdate_now},
    request, response,
    uri::HttpScheme,
};

mod context;
mod body;
mod error;

pub use context::HttpContext;
pub use error::ProtoError;

pub(crate) const MAX_HEADERS: usize = 64;

#[derive(Debug)]
pub struct HttpState {
    reqline: Reqline,
    headers: HeaderMap,
}

impl HttpState {
    pub fn new(reqline: Reqline) -> Self {
        Self::with_headers(reqline, HeaderMap::with_capacity(8))
    }

    pub fn with_headers(reqline: Reqline, headers: HeaderMap) -> Self {
        Self { reqline, headers }
    }

    pub fn insert_header(&mut self, mut header: Header) -> Result<(), ProtoError> {
        if self.headers.len() > MAX_HEADERS {
            return Err(ProtoError::TooManyHeaders);
        }

        header.value.make_ascii_lowercase();

        self.headers.append(
            HeaderName::from_slice(header.name).expect("TODO"),
            HeaderValue::from_slice(header.value.freeze()).expect("TODO"),
        );

        Ok(())
    }

    pub fn try_content_len(&self) -> Result<Option<u64>, HeaderError> {
        match self.headers.get(CONTENT_LENGTH) {
            Some(content_len) => match tcio::atou(content_len.as_bytes()) {
                Some(ok) => Ok(Some(ok)),
                None => todo!("to be removed"),
            },
            None => Ok(None),
        }
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

pub fn write_response(res: &response::Parts, buf: &mut BytesMut, content_len: u64) {
    buf.reserve(128);

    buf.extend_from_slice(res.version.as_str().as_bytes());
    buf.extend_from_slice(b" ");
    buf.extend_from_slice(res.status.as_str().as_bytes());
    buf.extend_from_slice(b"\r\nDate: ");
    buf.extend_from_slice(&httpdate_now()[..]);
    buf.extend_from_slice(b"\r\nContent-Length: ");
    buf.extend_from_slice(itoa::Buffer::new().format(content_len).as_bytes());
    buf.extend_from_slice(b"\r\n");

    for (key, val) in &res.headers {
        buf.extend_from_slice(key.as_str().as_bytes());
        buf.extend_from_slice(b": ");
        buf.extend_from_slice(val.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    buf.extend_from_slice(b"\r\n");
}
