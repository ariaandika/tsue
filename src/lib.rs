//! # Server and Client Toolkit
//!
//! This library provide a toolkit for building a server and client for various different
//! protocols.
//!
//! # Library Design
//!
//! This library is design so that it can be used as building block for writing a server.
//! Additionally, it also provide a ready to use API that combine all components to run a server.
//! It can also be used as an example to use and integrate each available components.
//!
//! ## Definitions
//!
//! - [`uri`] Uniform Resource Identifier ([RFC3986])
//! - [`headers`] HTTP Header Fields ([RFC9110 Section 5])
//! - [`http`] HTTP Semantics ([RFC9110])
//!
//! ## Behaviors
//!
//! - [`h1`] HTTP/1.1 ([RFC9112])
//! - [`h2`] HTTP/2.0 (RFC9113)
//!
//! ## User Abstraction
//!
//! - [`service`] abstract user defined logic
//!
//! ## Integrations
//!
//! - [`server`] all in one API to run a http server
//!
//! # Usage
//!
//! User can use each APIs individually to build custom server, or use available APIs from
//! [`server`] to quickly run a server.
//!
//! [RFC3986]: <https://www.rfc-editor.org/rfc/rfc3986.html>
//! [RFC9110]: <https://www.rfc-editor.org/rfc/rfc9110.html>
//! [RFC9110 Section 5]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-fields>
//! [RFC9110 Section 6]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-message-abstraction>
//! [RFC9112]: <https://www.rfc-editor.org/rfc/rfc9112.html>
#![warn(missing_debug_implementations)]

mod log;
mod matches;
pub mod common;

// definitions
pub mod uri;
pub mod headers;
pub mod http;
pub mod body;

// HTTP protocol
pub mod proto;
pub mod h1;
pub mod h2;

// user abstraction
pub mod service;

// integration
pub mod server;
