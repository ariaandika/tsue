// TE - transfer-encoding preferences
// Expect - 100-continue handling
// Range - partial content requests
// Trailer - trailing headers after chunked body
// Priority - HTTP/2/3 stream prioritization
use crate::h1::parser::Reqline;
use crate::headers::{HeaderMap, standard::CONNECTION};
use crate::http::{Method, Version};
use crate::proto::error::ProtoError;

// TODO: protocol upgrade www.rfc-editor.org/rfc/rfc9110.html#name-upgrade

#[derive(Debug)]
pub struct HttpContext {
    pub is_keep_alive: bool,
    pub is_res_body_allowed: bool,
}

impl Default for HttpContext {
    fn default() -> Self {
        Self {
            is_keep_alive: false,
            is_res_body_allowed: false,
        }
    }
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
