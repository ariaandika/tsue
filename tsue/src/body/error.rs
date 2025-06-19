use std::{backtrace::Backtrace, fmt};

use super::limited::LengthLimitError;
use crate::response::{IntoResponse, Response};

pub struct BodyError {
    kind: Kind,
    backtrace: Backtrace,
}

impl BodyError {
    /// Returns the underlying [`Backtrace`].
    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl From<Kind> for BodyError {
    fn from(kind: Kind) -> Self {
        Self { kind, backtrace: Backtrace::capture() }
    }
}

#[derive(Debug)]
pub enum Kind {
    Incoming(hyper::Error),
    Limited(LengthLimitError),
}

impl From<hyper::Error> for BodyError {
    fn from(v: hyper::Error) -> Self {
        Self::from(Kind::Incoming(v))
    }
}

impl From<LengthLimitError> for BodyError {
    fn from(v: LengthLimitError) -> Self {
        Self::from(Kind::Limited(v))
    }
}

impl IntoResponse for BodyError {
    fn into_response(self) -> Response {
        match self.kind {
            Kind::Incoming(r) => r.into_response(),
            Kind::Limited(l) => l.into_response(),
        }
    }
}

impl std::error::Error for BodyError { }

impl fmt::Debug for BodyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_tuple("BodyError");
        match &self.kind {
            Kind::Incoming(r) => f.field(&r),
            Kind::Limited(l) => f.field(&l),
        }.finish()
    }
}

impl fmt::Display for BodyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            Kind::Incoming(r) => r.fmt(f)?,
            Kind::Limited(l) => l.fmt(f)?,
        }

        if let std::backtrace::BacktraceStatus::Captured = self.backtrace.status() {
            let backtrace = self.backtrace.to_string();
            writeln!(f, "\n\nBodyError stack backtrace:")?;
            write!(f, "{}", backtrace.trim_end())?;
        }

        Ok(())
    }
}
