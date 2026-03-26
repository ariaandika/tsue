//! HTTP Semantics ([RFC9110])
//!
//! [RFC9110]: <https://www.rfc-editor.org/rfc/rfc9110>
mod method;
mod status;
mod version;
mod date;
mod scheme;
mod host;
mod target;
pub mod request;
pub mod response;

pub use method::Method;
pub use version::Version;
pub use status::StatusCode;
pub use date::{httpdate, httpdate_now};
pub use scheme::Scheme;
pub use host::Host;
pub use target::Target;
pub use request::Request;
pub use response::Response;

pub mod error {
    //! Error types.
    pub use super::method::UnknownMethod;
}

