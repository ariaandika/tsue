use crate::request::Request;
use http::Method;

/// Partially match request.
///
/// Use [`PartialEq`]
#[derive(Clone, Default)]
pub struct Matcher {
    path: Option<&'static str>,
    method: Option<Method>,
}

impl PartialEq<Request> for Matcher {
    fn eq(&self, other: &Request) -> bool {
        if let Some(path) = self.path {
            if path != other.uri().path() {
                return false;
            }
        }
        if let Some(method) = &self.method {
            if method != other.method() {
                return false;
            }
        }
        true
    }
}

macro_rules! matcher_from {
    ($id:pat,$ty:ty => $($tt:tt)*) => {
        impl From<$ty> for Matcher {
            fn from($id: $ty) -> Self {
                Self $($tt)*
            }
        }
    };
}

matcher_from!(_,() => { path: None, method: None });
matcher_from!(value,Method => { method: Some(value), path: None });
matcher_from!(value,&'static str => { path: Some(value), method: None });
matcher_from!((p,m),(&'static str,Method) => { path: Some(p), method: Some(m) });
