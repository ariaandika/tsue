use tcio::bytes::{ByteStr, BytesMut};

use crate::{
    h1::parser::{error::ErrorKind, HttpError},
    http::Method,
    uri::{Authority, HttpUri, Path, Scheme, Uri},
};

#[derive(Debug)]
pub struct Target {
    pub value: BytesMut,
    pub kind: Kind,
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

    pub fn build_origin(self, host: ByteStr, scheme: Scheme) -> Result<Uri, HttpError> {
        let authority;
        let path;

        match self.kind {
            Kind::Origin => {
                authority = Authority::try_from(host)?;
                path = Path::try_from(self.value)?;
            },
            Kind::Absolute => {
                let _ = HttpUri::parse_http(self.value.freeze())?;
                todo!()
            },
            Kind::Asterisk => {
                authority = Authority::try_from(host)?;
                path = Path::asterisk();
            },
            Kind::Authority => {
                if self.value.as_slice() != host.as_bytes() {
                    return Err(ErrorKind::MissmatchHost.into());
                }
                authority = Authority::try_from(self.value)?;
                path = Path::empty();
            },
        }

        Ok(Uri::from_parts(scheme, Some(authority), path))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Asterisk,
    Origin,
    Absolute,
    Authority,
}

