// TE - transfer-encoding preferences
// Expect - 100-continue handling
// Range - partial content requests
// Trailer - trailing headers after chunked body
// Priority - HTTP/2/3 stream prioritization
use crate::{
    h1::{parser::Reqline, spec::ProtoError},
    headers::{HeaderMap, standard::CONNECTION},
    http::{Method, Version},
};

// macro_rules! err {
//     ($variant:ident) => {
//         super::ProtoError::from(super::ProtoErrorKind::$variant)
//     };
// }

// TODO: protocol upgrade www.rfc-editor.org/rfc/rfc9110.html#name-upgrade

#[derive(Debug)]
pub struct HttpContext {
    pub is_keep_alive: bool,
    pub is_res_body_allowed: bool,
}

impl HttpContext {
    pub fn new(reqline: &Reqline, headers: &HeaderMap) -> Result<Self, ProtoError> {
        let mut is_keep_alive = matches!(
            reqline.version,
            Version::HTTP_11 | Version::HTTP_2 | Version::HTTP_3
        );
        let is_res_body_allowed = !matches!(reqline.method, Method::HEAD);

        if let Some(value) = headers.get(CONNECTION) {
            for conn in value.as_bytes().split(|&e| e == b',') {
                if conn.eq_ignore_ascii_case(b"close") {
                    is_keep_alive = false;
                    break; // "close" is highest priority
                }
                if conn.eq_ignore_ascii_case(b"keep-alive") {
                    is_keep_alive = true;
                }
            }
        }

        Ok(Self {
            is_keep_alive,
            is_res_body_allowed,
        })
    }
}

// ===== Body =====

// rfc-editor.org/rfc/rfc9110.html#name-representation-data-and-met
//
// Content-Type - with boundary for multipart
// Content-Encoding - gzip, deflate, brotli
// Content-Length
// Transfer-Encoding - chunked, gzip, etc.
