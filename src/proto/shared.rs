use tcio::bytes::BytesMut;

use crate::http::{Method, Version};

#[derive(Debug)]
pub struct Reqline {
    pub method: Method,
    pub target: BytesMut,
    pub version: Version,
}

#[derive(Debug)]
pub struct Header {
    pub name: BytesMut,
    pub value: BytesMut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Asterisk,
    Origin,
    Absolute,
    Authority,
}

impl TargetKind {
    /// Get the target kind of a request line.
    pub fn new(method: &Method, target: &[u8]) -> Self {
        match target {
            [b'/', ..] => Self::Origin,
            b"*" => Self::Asterisk,
            _ => {
                if method != &Method::CONNECT {
                    Self::Absolute
                } else {
                    Self::Authority
                }
            }
        }
    }
}
