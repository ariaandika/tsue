//! Web Server and Client Toolkit
//!
//! - [`uri`] Uniform Resource Identifier ([RFC3986])
//! - [`http`] HTTP Semantics ([RFC9110])
//! - [`h1`] HTTP/1.1 ([RFC9112])
//!
//! [RFC3986]: <https://datatracker.ietf.org/doc/html/rfc3986>
//! [RFC9110]: <https://datatracker.ietf.org/doc/html/rfc9110>
//! [RFC9112]: <https://datatracker.ietf.org/doc/html/rfc9112>
#![warn(missing_debug_implementations)]

mod matches;

pub mod headers;
pub mod http;
pub mod uri;

pub mod body;
pub mod h1;
pub mod request;
pub mod response;

pub mod server;
pub mod service;
