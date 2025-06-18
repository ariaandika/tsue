use http::Uri;
use crate::request::Request;

#[derive(Debug)]
pub struct Path {
    value: &'static str,
    is_params: bool,
}

impl Path {
    pub fn new(value: &'static str) -> Self {
        Self { value, is_params: value.contains(':') || value.contains('*') }
    }

    pub fn matches(&self, req: &Request) -> bool {
        if !self.is_params {
            return req.uri().path() == self.value;
        }

        let mut p1 = req.uri().path().split('/');
        let mut p2 = self.value.split('/');

        loop {
            match (p1.next(), p2.next()) {
                (None, None) => return true,
                (Some(p1), Some(p2)) if p2.starts_with(':') => {
                    if p1.is_empty() {
                        return false;
                    }
                }
                (Some(_), Some("*")) => {}
                (Some(p1), Some(p2)) => if p1 != p2 { return false },
                _ => return false,
            }
        }
    }

    pub fn value(&self) -> &'static str {
        self.value
    }
}

// ===== Internals =====

pub(crate) trait RequestInternal {
    fn with_prefixed(self, prefix: &str) -> Self;
}

impl RequestInternal for Request {
    fn with_prefixed(mut self, prefix: &str) -> Self {
        let trimmed = match self.uri().path_and_query() {
            Some(p) => p.as_str(),
            None => self.uri().path(),
        }
        .trim_start_matches(prefix);

        // TODO: set original path extension
        // let _original = self.uri().path();

        let mut parts = self.uri().clone().into_parts();
        parts.path_and_query = Some(trimmed.parse().expect("cloned from valid Uri"));
        *self.uri_mut() = Uri::from_parts(parts).expect("cloned from valid Uri");

        self
    }
}

