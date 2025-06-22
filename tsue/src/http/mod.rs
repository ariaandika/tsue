mod method;
mod path;

pub use method::Method;
pub use path::PathAndQuery;

#[derive(Debug)]
pub struct Parts {
    pub method: Method,
    pub path: PathAndQuery,
}

