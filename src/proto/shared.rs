use tcio::bytes::BytesMut;

use crate::h1::parser::Target;
use crate::http::{Method, Version};

#[derive(Debug)]
pub struct Reqline {
    pub method: Method,
    pub target: Target,
    pub version: Version,
}

#[derive(Debug)]
pub struct Header {
    pub name: BytesMut,
    pub value: BytesMut,
}

