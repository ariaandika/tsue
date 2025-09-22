//! Web Server and Client Toolkit
#![warn(missing_debug_implementations)]

mod matches;
pub mod uri;
pub mod h1;

pub mod http;
pub mod headers;
pub mod body;
pub mod request;
pub mod response;

pub mod service;
pub mod server;

