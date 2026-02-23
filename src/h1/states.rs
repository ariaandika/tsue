use crate::body::shared::SendHandle;
use crate::headers::HeaderMap;
use crate::uri::HttpScheme;

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
