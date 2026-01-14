use crate::{h1::parser::Target, http::{Method, Version}};


#[derive(Debug)]
pub struct Reqline {
    pub method: Method,
    pub target: Target,
    pub version: Version,
}

