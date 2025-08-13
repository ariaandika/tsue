//! Web Server and Client Toolkit
#![warn(missing_debug_implementations)]

pub mod http;
pub mod headers;
pub mod body;
pub mod request;
pub mod response;

mod parser;
mod proto;

pub mod service;
pub mod server;

