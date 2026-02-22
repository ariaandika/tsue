use crate::body::shared::SendHandle;
use crate::headers::HeaderMap;
use crate::http::Method;
use crate::uri::HttpScheme;

#[derive(Debug, Clone)]
pub struct Context {
    pub method: Method,
}

impl Context {
    /// https://www-rfc-editor.org/rfc/rfc9110.html#section-6.4.2-4
    pub fn is_res_body_allowed(&self) -> bool {
        !matches!(self.method, Method::HEAD)
    }
}

#[derive(Debug)]
pub struct Session {
    pub scheme: HttpScheme,
    pub headers: HeaderMap,
    pub shared: SendHandle,
    pub keep_alive: bool,
}

impl Session {
    pub fn new() -> Self {
        Self {
            scheme: HttpScheme::HTTP,
            headers: HeaderMap::with_capacity(32),
            shared: SendHandle::new(),
            keep_alive: true,
        }
    }
}
