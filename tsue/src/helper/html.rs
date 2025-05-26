use bytes::Bytes;
use http::{HeaderName, HeaderValue, header::CONTENT_TYPE};

use super::{Html, macros::derefm};
use crate::response::{IntoResponse, Response};

derefm!(<T>|Html<T>| -> T);

const TEXT_HTML: [(HeaderName, HeaderValue); 1] = [(
    CONTENT_TYPE,
    HeaderValue::from_static("text/html; charset=utf-8"),
)];

impl<T: Into<Bytes>> IntoResponse for Html<T> {
    fn into_response(self) -> Response {
        (TEXT_HTML, self.0.into()).into_response()
    }
}
