use crate::request::Request;

/// Internal state for routing.
#[derive(Default)]
pub struct Shared {
    pub(crate) path_offset: u32,
}

#[derive(Debug)]
pub struct Path {
    repr: Repr,
}

#[derive(Debug)]
enum Repr {
    Static(&'static str),
    Params(&'static str),
}

impl Path {
    pub fn new(value: &'static str) -> Self {
        if value.contains(':') || value.contains('*') {
            Self { repr: Repr::Params(value) }
        } else {
            Self { repr: Repr::Static(value) }
        }
    }

    pub fn matches(&self, req: &Request) -> bool {
        let path = match self.repr {
            Repr::Static(p) => return req.matches_path() == p,
            Repr::Params(p) => p,
        };

        let mut p1 = req.matches_path().split('/');
        let mut p2 = path.split('/');

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
}

// ===== Internals =====

pub(crate) trait RequestInternal {
    fn matches_path(&self) -> &str;
}

impl RequestInternal for Request {
    fn matches_path(&self) -> &str {
        let path = self.uri().path().split_at(self.body().shared().path_offset as _).1;
        if path.is_empty() {
            "/"
        } else {
            path
        }
    }
}

