use std::mem;
use std::task::Poll::{self, *};
use tcio::bytes::{Bytes, BytesMut};
use tcio::num::itoa;

use crate::body::{Body, Incoming};
use crate::h1::body::{BodyKind, H1BodyDecoder};
use crate::h1::chunked::ChunkedCoder;
use crate::h1::parser::{find_crlf, parse_header, parse_reqline};
use crate::h1::states::{Context, Session};
use crate::headers::{HeaderField, HeaderName, HeaderValue, standard};
use crate::http::{Method, Request, Response, httpdate_now, request, response};
use crate::proto::error::{ParseError, ProtoError};
use crate::uri::{Host, HttpUri, Path};

pub(crate) struct RequestParser {
    reqline: Option<(Method, Bytes)>,
}

impl RequestParser {
    pub(crate) fn new() -> Self {
        Self { reqline: None }
    }

    pub(crate) fn poll_request(
        &mut self,
        session: &mut Session,
        read_buffer: &mut BytesMut,
    ) -> Poll<Result<(Method, Bytes), ProtoError>> {

        if self.reqline.is_none() {
            let Some(reqline) = find_crlf(read_buffer) else {
                return Pending;
            };
            self.reqline = Some(parse_reqline(reqline)?);
        }

        let mut len = 0u8;
        loop {
            if len > 64 {
                return Ready(Err(ProtoError::TooManyHeaders));
            }
            let Some(line) = find_crlf(read_buffer) else {
                return Pending;
            };
            if line.is_empty() {
                break;
            }
            let (name, val) = parse_header(line)?;
            let (name, hash) = HeaderName::from_internal(name)?;
            let val = HeaderValue::from_bytes(val)?;
            let field = HeaderField::with_hash(name, val, hash);

            // cannot returns error, the len is capped at 64
            let _ = session.headers.try_append_field(field);

            len += 1;
        }

        Ready(Ok(self.reqline.take().expect("checked")))
    }
}

// ===== Service Manager =====

#[derive(Debug)]
pub(crate) struct RequestState {
    context: Context,
    decoder: H1BodyDecoder,
}

impl RequestState {
    pub(crate) fn new(
        method: Method,
        target: Bytes,
        session: &mut Session,
        read_buffer: &mut BytesMut,
        cx: &mut std::task::Context,
    ) -> Result<(Request<Incoming>, Self), ProtoError> {
        let headers = mem::take(&mut session.headers);

        let decoder = H1BodyDecoder::new(&headers).expect("TODO");
        let context = Context { method };

        let host = match headers.get(standard::HOST) {
            Some(value) => Bytes::from(value.clone()),
            None => return Err(ProtoError::MissingHost),
        };

        let uri_host;
        let path;

        match target.as_slice() {
            [b'/', ..] => {
                // origin
                uri_host = Host::from_bytes(host)?;
                path = Path::from_bytes(target)?;
            }
            b"*" => {
                // asterisk
                uri_host = Host::from_bytes(host)?;
                path = Path::from_static(b"*");
            }
            _ => if method != Method::CONNECT {
                // absolute
                let uri = HttpUri::from_bytes(target)?;
                if uri.host().as_bytes() == host.as_slice() {
                    return Err(ParseError::MissmatchHost.into());
                }
                let (_, h, p) = uri.into_parts();
                uri_host = h;
                path = p;
            } else {
                // auth
                if target != host {
                    return Err(ParseError::MissmatchHost.into());
                }
                uri_host = Host::from_bytes(target)?;
                path = Path::from_static(b"");
            }
        }

        let uri = HttpUri::from_parts(session.scheme, uri_host, path);

        let parts = request::Parts {
            method,
            uri,
            version: crate::http::Version::HTTP_11,
            headers,
            extensions: crate::http::Extensions::new(),
        };
        let body = decoder.build_body(read_buffer, &mut session.shared, cx);
        let request = Request::from_parts(parts, body);

        Ok((request, Self { context, decoder }))
    }

    /// Poll for request body, returns `true` if more read is required.
    ///
    /// This should be polled with the `Service` future.
    pub(crate) fn poll_read(
        &mut self,
        session: &mut Session,
        read_buffer: &mut BytesMut,
        cx: &mut std::task::Context,
    ) -> bool {
        session
            .shared
            .poll_read(read_buffer, &mut self.decoder, cx)
            .is_pending()
    }

    pub(crate) fn build_response_writer<B>(
        &self,
        response: Response<B>,
        session: &mut Session,
        write_buffer: &mut BytesMut,
    ) -> Option<(B, BodyKind)>
    where
        B: Body,
    {
        let (parts, body) = response.into_parts();
        let size_hint = body.size_hint();
        let clen = size_hint.1.filter(|&l|l == size_hint.0);

        write_response_head(&parts, &mut *write_buffer, clen);

        // reuse header map allocation
        let mut headers = parts.headers;
        headers.clear();
        session.headers = headers;

        let context = self.context.clone();

        if !context.is_res_body_allowed() {
            return None;
        }

        match clen {
            Some(len) => Some((body, BodyKind::ContentLength(len))),
            None => Some((body, BodyKind::Chunked(ChunkedCoder::new()))),
        }
    }
}

// ===== Response Writer =====

fn write_response_head(res: &response::Parts, buf: &mut BytesMut, content_length: Option<u64>) {
    buf.extend_from_slice(res.version.as_str().as_bytes());
    buf.extend_from_slice(b" ");
    buf.extend_from_slice(res.status.as_str().as_bytes());
    buf.extend_from_slice(b"\r\nDate: ");
    buf.extend_from_slice(&httpdate_now()[..]);

    match content_length {
        Some(len) => {
            buf.extend_from_slice(b"\r\nContent-Length: ");
            buf.extend_from_slice(itoa().format(len).as_bytes());
            buf.extend_from_slice(b"\r\n");
        },
        None => {
            // FEAT: support compressed transfer-encodings
            buf.extend_from_slice(b"\r\nTransfer-Encoding: chunked\r\n");
        },
    }

    for f in &res.headers {
        buf.extend_from_slice(f.name().as_str().as_bytes());
        buf.extend_from_slice(b": ");
        buf.extend_from_slice(f.value().as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    buf.extend_from_slice(b"\r\n");
}

