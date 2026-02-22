use std::mem;
use std::task::Poll::{self, *};
use tcio::bytes::{Bytes, BytesMut};

use crate::body::shared::SendHandle;
use crate::body::{Body, BodyCoder, Codec, Incoming};
use crate::h1::chunked::ChunkedCoder;
use crate::h1::parser::{find_crlf, parse_header, parse_reqline};
use crate::headers::{HeaderField, HeaderMap, HeaderName, HeaderValue, standard};
use crate::http::{Method, Request, Response, httpdate_now, request, response};
use crate::proto::error::{ParseError, ProtoError};
use crate::uri::{Host, HttpScheme, HttpUri, Path};

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
    decoder: BodyCoder,
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

        let decoder = BodyCoder::new(&headers).expect("TODO");
        let context = Context::new(method, &headers)?;

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
    ) -> (B, BodyKind)
    where
        B: Body,
    {
        let (parts, body) = response.into_parts();
        let is_res_body = !body.is_end_stream();

        let encoder = BodyCoder::from_len(body.size_hint().1);
        let coding = is_res_body.then_some(encoder.coding());

        write_response_head(&parts, &mut *write_buffer, coding);

        // reuse header map allocation
        let mut headers = parts.headers;
        headers.clear();
        session.headers = headers;

        let context = self.context.clone();

        if context.is_res_body_allowed && is_res_body {
            let (_, upper) = body.size_hint();
            match upper {
                Some(length) => (body, BodyKind::Exact(length)),
                None => (body, BodyKind::Chunked(ChunkedCoder::new())),
            }
        } else {
            (body, BodyKind::None)
        }
    }
}

// ===== Response Writer =====

fn write_response_head(res: &response::Parts, buf: &mut BytesMut, coding: Option<Codec>) {
    buf.extend_from_slice(res.version.as_str().as_bytes());
    buf.extend_from_slice(b" ");
    buf.extend_from_slice(res.status.as_str().as_bytes());
    buf.extend_from_slice(b"\r\nDate: ");
    buf.extend_from_slice(&httpdate_now()[..]);

    if let Some(coding) = coding {
        match coding {
            Codec::Chunked => {
                // FEAT: support compressed transfer-encodings
                buf.extend_from_slice(b"\r\nTransfer-Encoding: chunked\r\n");
            }
            Codec::ContentLength(len) => {
                buf.extend_from_slice(b"\r\nContent-Length: ");
                buf.extend_from_slice(itoa::Buffer::new().format(len).as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
        }
    }

    for f in &res.headers {
        buf.extend_from_slice(f.name().as_str().as_bytes());
        buf.extend_from_slice(b": ");
        buf.extend_from_slice(f.value().as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    buf.extend_from_slice(b"\r\n");
}

pub(crate) enum BodyKind {
    None,
    Exact(u64),
    Chunked(ChunkedCoder),
}

// ===== Context =====

#[derive(Debug, Clone)]
struct Context {
    is_keep_alive: bool,
    is_res_body_allowed: bool,
}

impl Context {
    fn new(method: Method, headers: &HeaderMap) -> Result<Self, ProtoError> {
        // https://www-rfc-editor.org/rfc/rfc9110.html#section-6.4.2-4
        let is_res_body_allowed = !matches!(method, Method::HEAD);

        let mut is_keep_alive = true;

        if let Some(value) = headers.get(standard::CONNECTION) {
            for conn in value.as_bytes().split(|&e| e == b',') {
                if conn.eq_ignore_ascii_case(b"close") {
                    is_keep_alive = false;
                    break; // "close" is highest priority
                }
                if conn.eq_ignore_ascii_case(b"keep-alive") {
                    is_keep_alive = true;
                }
            }
        }

        Ok(Self {
            is_keep_alive,
            is_res_body_allowed,
        })
    }
}

// ===== Session =====

#[derive(Debug)]
pub(crate) struct Session {
    scheme: HttpScheme,
    headers: HeaderMap,
    shared: SendHandle,
}

impl Session {
    pub(crate) fn new() -> Self {
        Self {
            scheme: HttpScheme::HTTP,
            headers: HeaderMap::with_capacity(32),
            shared: SendHandle::new(),
        }
    }
}
