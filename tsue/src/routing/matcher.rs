use http::Method;

use crate::request::Request;

/// Helper trait to conviniently setup route matcher.
///
/// Implemented for:
/// - `&'static str`
/// - `Method`
/// - `(&'static str, Method)`
/// - `(Method, &'static str)`
pub trait Matcher {
    fn matcher(self) -> (Option<Method>,Option<&'static str>);
}

macro_rules! impl_matcher {
    ($me:ty,$id:ident => $body:expr) => {
        impl Matcher for $me {
            fn matcher($id) -> (Option<Method>,Option<&'static str>) {
                $body
            }
        }
    };
}

impl_matcher!(&'static str, self => (None,Some(self)));
impl_matcher!(Method, self => (Some(self),None));
impl_matcher!((Method, &'static str), self => (Some(self.0),Some(self.1)));
impl_matcher!((&'static str, Method), self => (Some(self.1),Some(self.0)));

// ===== Internals =====

pub(crate) trait RequestInternal {
    fn match_path(&self) -> &str;
}

impl RequestInternal for Request {
    fn match_path(&self) -> &str {
        // PERF: accessing `extensions` in hot code path, especially O(n) of routes count, may have
        // performance hit

        match self.extensions().get::<Matched>() {
            Some(m) => self.uri().path().split_at(m.midpoint as _).1,
            None => self.uri().path(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Matched {
    pub(crate) midpoint: u32,
}

