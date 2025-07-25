//! Web Server and Client Toolkit
#![warn(missing_debug_implementations)]

pub use tcio::ByteStr;

pub mod http;
pub mod headers;
pub mod body;
pub mod request;
pub mod response;

pub mod parser;
pub mod proto;
pub mod service;

pub mod server;

