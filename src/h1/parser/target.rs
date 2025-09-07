use tcio::bytes::{ByteStr, BytesMut};

use crate::{
    h1::parser::{HttpError, error::ErrorKind},
    http::Method,
    uri::{Authority, Path},
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

    pub fn build_origin(self, host: ByteStr, is_https: bool) -> Result<HttpUri, HttpError> {
        match self.kind {
            Kind::Origin => Ok(HttpUri {
                is_https,
                authority: Authority::try_from(host)?,
                path: Path::try_from(self.value)?,
            }),
            Kind::Absolute => Ok(todo!()),
            Kind::Asterisk => Ok(HttpUri {
                is_https,
                authority: Authority::try_from(host)?,
                path: Path::asterisk(),
            }),
            Kind::Authority => {
                if self.value.as_slice() != host.as_bytes() {
                    return Err(ErrorKind::MissmatchHost.into());
                }
                Ok(HttpUri {
                    is_https,
                    authority: Authority::try_from(self.value)?,
                    path: Path::empty(),
                })
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Asterisk,
    Origin,
    Absolute,
    Authority,
}

// ===== URI =====

#[derive(Debug)]
pub struct HttpUri {
    is_https: bool,
    authority: Authority,
    path: Path,
}

