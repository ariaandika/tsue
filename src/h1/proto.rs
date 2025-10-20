//! HTTP/1.1 Logic.
use tcio::bytes::{Bytes, BytesMut};

use super::parser::{Header, Reqline};
use crate::{
    headers::{HeaderMap, HeaderName, HeaderValue},
    http::{Extensions, httpdate_now},
    request, response,
    uri::HttpScheme,
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const MAX_HEADERS: usize = 64;

#[derive(Debug)]
pub struct HttpState {
    reqline: Reqline,
    headers: HeaderMap,
    host: Option<Bytes>,
    content_len: Option<u64>,
}

impl HttpState {
    pub fn new(reqline: Reqline) -> Self {
        Self {
            reqline,
            headers: HeaderMap::with_capacity(8),
            host: None,
            content_len: None,
        }
    }

    pub fn with_cached_headers(reqline: Reqline, headers: HeaderMap) -> Self {
        debug_assert!(headers.is_empty());
        Self {
            reqline,
            headers,
            host: None,
            content_len: None,
        }
    }

    pub fn add_header(&mut self, mut header: Header) -> Result<(), BoxError> {
        if self.headers.len() > MAX_HEADERS {
            return Err("too many headers".into());
        }

        let name = HeaderName::from_slice(header.name)?;

        header.value.make_ascii_lowercase();
        let value = header.value.freeze();

        match name.as_str() {
            "content-length" => {
                match tcio::atou(&value) {
                    Some(ok) => self.content_len = Some(ok),
                    None => return Err("invalid content-length".into()),
                }
            },
            "host" => {
                self.host = Some(value.clone());
            },
            _ => {},
        }

        self.headers
            .insert(name, HeaderValue::try_from_slice(value)?);

        Ok(())
    }

    pub fn content_len(&self) -> Option<u64> {
        self.content_len
    }

    pub fn build_parts(self) -> Result<request::Parts, BoxError> {
        let Some(host) = self.host else {
            return Err("missing host header".into());
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
