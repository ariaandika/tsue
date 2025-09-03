use tcio::{ByteStr, bytes::Bytes};

use super::parser::{Header, Reqline};
use crate::{
    headers::{HeaderMap, HeaderName, HeaderValue},
    http::{Extensions, Uri},
    request::Parts,
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

    pub fn add_header(&mut self, header: Header) -> Result<(), BoxError> {
        if self.headers.len() > MAX_HEADERS {
            return Err("too many headers".into());
        }

        let name = HeaderName::new(ByteStr::from_utf8(header.name.freeze())?);
        let value = header.value.freeze();

        if name.as_str().eq_ignore_ascii_case("content-length") {
            match tcio::atou(&value) {
                Some(ok) => self.content_len = Some(ok),
                None => return Err("invalid content-length".into()),
            }
        }

        if name.as_str().eq_ignore_ascii_case("host") {
            self.host = Some(value.clone());
        }

        self.headers
            .insert(name, HeaderValue::try_from_slice(value)?);

        Ok(())
    }

    pub fn content_len(&self) -> Option<u64> {
        self.content_len
    }

    pub fn build_parts(self) -> Result<Parts, BoxError> {
        // TODO: reconstruct URI from a complete Request
        // https://httpwg.org/specs/rfc9112.html#reconstructing.target.uri

        Ok(Parts {
            method: self.reqline.method,
            uri: Uri::http_root(), // TODO: URI path only parsing
            version: self.reqline.version,
            headers: self.headers,
            extensions: Extensions::new(),
        })
    }
}
