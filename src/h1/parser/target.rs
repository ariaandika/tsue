use tcio::bytes::{Bytes, BytesMut};

use crate::{
    h1::parser::{HttpError, error::ErrorKind},
    http::Method,
    uri::{Authority, HttpUri, Path},
};

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

    pub fn build_origin(self, host: Bytes, is_https: bool) -> Result<HttpUri, HttpError> {
        let authority;
        let path;

        match self.kind {
            Kind::Origin => {
                authority = Authority::parse_from(host)?;
                path = Path::parse_from(self.value)?;
            },
            Kind::Absolute => {
                let uri = HttpUri::parse_from(self.value)?;
                if uri.authority().as_bytes() == host.as_slice() {
                    return Err(ErrorKind::MissmatchHost.into());
                }
                return Ok(uri);
            },
            Kind::Asterisk => {
                authority = Authority::parse_from(host)?;
                path = Path::from_static(b"*");
            },
            Kind::Authority => {
                if self.value != host {
                    return Err(ErrorKind::MissmatchHost.into());
                }
                authority = Authority::parse_from(self.value)?;
                path = Path::from_static(b"");
            },
        }

        Ok(HttpUri::from_parts(is_https, authority, path))
    }
}

