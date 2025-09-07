use tcio::{ByteStr, bytes::BytesMut};

use crate::{
    http::Method,
    uri::{Authority, Path, UriError},
};

#[derive(Debug)]
pub struct Target {
    pub value: BytesMut,
    pub kind: Kind,
}

impl Target {
    pub(crate) fn new(method: &Method, target: BytesMut) -> Self {
        let kind = if method == &Method::CONNECT {
            Kind::Authority
        } else {
            match target.as_slice() {
                b"*" => Kind::Asterisk,
                [b'/', ..] => Kind::Origin,
                _ => Kind::Absolute,
            }
        };

        Target {
            value: target,
            kind,
        }
    }

    pub fn build_origin(self, host: ByteStr, is_https: bool) -> Result<HttpUri, UriError> {
        match self.kind {
            Kind::Asterisk => Ok(HttpUri {
                is_https,
                authority: Authority::try_from(host)?,
                path: Path::asterisk(),
            }),
            Kind::Origin => Ok(HttpUri {
                is_https,
                authority: Authority::try_from(host)?,
                path: Path::try_from(self.value.freeze())?,
            }),
            Kind::Absolute => Ok(todo!()),
            Kind::Authority => {
                // TODO: checks `host` with uri target
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

