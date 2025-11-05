//! Web Server and Client Toolkit
//!
//! - [`uri`] Uniform Resource Identifier ([RFC3986])
//! - [`http`] HTTP Semantics ([RFC9110])
//! - [`headers`] HTTP Header Fields ([RFC9110 Section 5])
//! - [`request`] HTTP Request Message ([RFC9110 Section 6])
//! - [`response`] HTTP Response Message Fields ([RFC9110 Section 6])
//! - [`h1`] HTTP/1.1 ([RFC9112])
//!
//! [RFC3986]: <https://www.rfc-editor.org/rfc/rfc3986.html>
//! [RFC9110]: <https://www.rfc-editor.org/rfc/rfc9110.html>
//! [RFC9110 Section 5]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-fields>
//! [RFC9110 Section 6]: <https://www.rfc-editor.org/rfc/rfc9110.html#name-message-abstraction>
//! [RFC9112]: <https://www.rfc-editor.org/rfc/rfc9112.html>
#![warn(missing_debug_implementations)]

mod matches;

// RFC3986
pub mod uri;

// RFC9110 Section 5
pub mod headers;

// RFC9110 Section 6
pub mod request;
// RFC9110 Section 6
pub mod response;

pub mod http;
pub mod body;
pub mod h1;
pub mod server;
pub mod service;
