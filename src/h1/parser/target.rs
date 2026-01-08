use tcio::bytes::{Bytes, BytesMut};

use super::ParseError;
use crate::http::Method;
use crate::uri::{Host, HttpScheme, HttpUri, Path};

#[derive(Debug)]
pub struct Target {
    pub value: BytesMut,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Asterisk,
    Origin,
    Absolute,
    Authority,
}

impl Target {
    pub(crate) fn new(method: &Method, target: BytesMut) -> Self {
        let kind = match target.as_slice() {
            [b'/', ..] => Kind::Origin,
            b"*" => Kind::Asterisk,
            _ => {
                if method != &Method::CONNECT {
                    Kind::Absolute
                } else {
                    Kind::Authority
                }
            }
        };

        Target {
            value: target,
            kind,
        }
    }

    pub fn build_origin(self, host: Bytes, scheme: HttpScheme) -> Result<HttpUri, ParseError> {
        let uri_host;
        let path;

        match self.kind {
            Kind::Origin => {
                uri_host = Host::from_bytes(host)?;
                path = Path::from_bytes(self.value)?;
            }
            Kind::Absolute => {
                let uri = HttpUri::from_bytes(self.value)?;
                if uri.host().as_bytes() == host.as_slice() {
                    return Err(ParseError::MissmatchHost);
                }
                return Ok(uri);
            }
            Kind::Asterisk => {
                uri_host = Host::from_bytes(host)?;
                path = Path::from_static(b"*");
            }
            Kind::Authority => {
                if self.value != host {
                    return Err(ParseError::MissmatchHost);
                }
                uri_host = Host::from_bytes(self.value)?;
                path = Path::from_static(b"");
            }
        }

        Ok(HttpUri::from_parts(scheme, uri_host, path))
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        tcio::fmt::lossy(&self.value).fmt(f)
    }
}

