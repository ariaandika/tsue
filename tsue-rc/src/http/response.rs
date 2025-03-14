//! the [`response::Parts`] and [`Response`] type
//!
//! [`response::Parts`]: Parts
use super::{Header, StatusCode, Version, MAX_HEADER};
use crate::{body::ResBody, bytestring::ByteStr};
use bytes::{Bytes, BytesMut};

#[derive(Default)]
pub struct Parts {
    version: Version,
    status: StatusCode,
    headers: [Header;MAX_HEADER],
    header_len: usize,
}

impl Parts {
    /// create new response parts
    pub fn new() -> Parts {
        Self::default()
    }

    /// return headers
    pub fn headers(&self) -> &[Header] {
        &self.headers[..self.header_len]
    }

    pub fn insert_header(&mut self, header: Header) {
        if self.header_len >= MAX_HEADER {
            return;
        }
        self.headers[self.header_len] = header;
        self.header_len += 1;
    }

    pub fn write(&self, bytes: &mut BytesMut) {
        bytes.extend_from_slice(self.version.as_bytes());
        bytes.extend_from_slice(b" ");
        bytes.extend_from_slice(self.status.as_bytes());
        bytes.extend_from_slice(b"\r\n");
        for Header { name, value } in self.headers() {
            bytes.extend_from_slice(name.as_bytes());
            bytes.extend_from_slice(b": ");
            bytes.extend_from_slice(&value);
            bytes.extend_from_slice(b"\r\n");
        }
        bytes.extend_from_slice(b"\r\n");
    }
}

#[derive(Default)]
pub struct Response {
    parts: Parts,
    body: ResBody,
}

impl Response {
    pub fn new(body: ResBody) -> Response {
        Response {
            parts: <_>::default(),
            body,
        }
    }

    /// perform a post write response
    ///
    /// - add httpdate
    /// - add content length
    pub fn check(&mut self) {
        self.parts.insert_header(Header {
            name: ByteStr::from_static(b"content-length"),
            value: Bytes::copy_from_slice(itoa::Buffer::new().format(self.body.len()).as_bytes()),
        });
    }

    pub fn write_headline(&self, bytes: &mut BytesMut) {
        self.parts.write(bytes);
    }

    pub fn into_parts(self) -> (Parts, ResBody) {
        (self.parts,self.body)
    }
}

