use http::{StatusCode, header::CONTENT_TYPE};

use crate::{
    request::Request,
    response::{IntoResponse, Response},
};

pub fn validate_content_type(req: &Request, content: &str) -> Option<()> {
    req.headers()
        .get(CONTENT_TYPE)?
        .to_str()
        .ok()?
        .contains(content)
        .then_some(())
}

#[derive(Debug)]
pub struct InvalidContentType;

impl std::error::Error for InvalidContentType {}

impl std::fmt::Display for InvalidContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unsupported media type")
    }
}

impl IntoResponse for InvalidContentType {
    fn into_response(self) -> Response {
        (StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported media type").into_response()
    }
}

