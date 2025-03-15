use super::{Parts, Response};
use crate::{bytestr::ByteStr, http::Header};
use bytes::{Bytes, BytesMut};


/// perform a post write response
///
/// - add httpdate
/// - add content length
pub fn check(res: &mut Response) {
    res.parts.insert_header(Header {
        name: ByteStr::from_static("content-length"),
        value: Bytes::copy_from_slice(itoa::Buffer::new().format(res.body.len()).as_bytes()),
    });
}

/// write http response parts into buffer
pub fn write(parts: &Parts, bytes: &mut BytesMut) {
    bytes.extend_from_slice(parts.version.as_bytes());
    bytes.extend_from_slice(b" ");
    bytes.extend_from_slice(parts.status.as_bytes());
    bytes.extend_from_slice(b"\r\n");
    for Header { name, value } in parts.headers() {
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(b": ");
        bytes.extend_from_slice(value);
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(b"\r\n");
}

