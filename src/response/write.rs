use bytes::BufMut;

use crate::response::Response;

pub fn write_head<B: BufMut>(res: &Response, mut bufm: B) {
    bufm.put_slice(res.parts.version.as_str().as_bytes());
    bufm.put_slice(b" ");
    bufm.put_slice(res.parts.status.as_str().as_bytes());
    bufm.put_slice(b"\r\n");

    for (name, value) in res.parts.headers.iter() {
        bufm.put_slice(name.as_str().as_bytes());
        bufm.put_slice(b": ");
        bufm.put_slice(value.as_bytes());
        bufm.put_slice(b"\r\n");
    }

    bufm.put_slice(b"\r\n");
}

