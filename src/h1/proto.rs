use std::mem;
use std::task::Poll::{self, *};
use tcio::bytes::{Buf, BytesMut};
use tcio::num::{itoa, wrapping_atou};

use crate::body::{Body, Incoming};
use crate::h1::body::{BodyDecoder, BodyEncoder, ContentKind};
use crate::h1::states::Session;
use crate::headers::{HeaderField, HeaderName, HeaderValue, standard};
use crate::http::{Method, Request, Response, httpdate_now, request, response};
use crate::matches;
use crate::proto::error::{ParseError, ProtoError, UserError};
use crate::uri::{Host, HttpUri, Path};

use ParseError as P;
use ProtoError as E;

const MIN_REQLINE_LEN: usize = b"GET / HTTP/1.1".len();

pub fn poll_request(
    session: &mut Session,
    read_buffer: &mut BytesMut,
) -> Poll<Result<RequestContext, ProtoError>> {
    let (reqline_len, mut state) = {
        if read_buffer.len() < MIN_REQLINE_LEN {
            return Pending;
        }
        let Some(lf) = matches::find_byte::<b'\n'>(read_buffer) else {
            return Pending;
        };
        if lf == 0 {
            return Ready(Err(P::InvalidSeparator.into()));
        }
        // gtfo
        if read_buffer[lf - 1] != b'\r' {
            return Ready(Err(P::InvalidSeparator.into()));
        }
        let len = lf + 1;
        (len, &read_buffer[len..])
    };

    const MAX_HEADERS: u8 = 32;
    let mut headers_len = 0u8;
    let mut headers = [const { mem::MaybeUninit::uninit() }; MAX_HEADERS as usize];
    loop {
        let Some(prefix) = state.first_chunk::<2>() else {
            return Pending;
        };
        match prefix[0] {
            b'\r' => if prefix[1] == b'\n' {
                state = &state[2..];
                break;
            } else {
                return Ready(Err(P::InvalidSeparator.into()));
            },
                b'\n' => return Ready(Err(P::InvalidSeparator.into())),
                _ => {}
        }
        let Some(lf) = matches::find_byte::<b'\n'>(state) else {
            return Pending;
        };
        if headers_len >= MAX_HEADERS {
            return Ready(Err(E::TooManyHeaders));
        }
        state = &state[lf + 1..];
        headers[headers_len as usize].write(u16::try_from(lf + 1).map_err(|_|P::TooLong)?);
        headers_len += 1;
    }

    let mut bytes = unsafe { read_buffer.split_to(state.as_ptr().offset_from_unsigned(read_buffer.as_ptr())) };

    // ===== Request Line =====

    let mut reqline = bytes.split_to(reqline_len);
    reqline.truncate(reqline_len - 2);

    let method;
    if reqline.first_chunk() == Some(b"GET ") {
        reqline.advance(4);
        method = Method::GET;
    } else if reqline.first_chunk() == Some(b"POST ") {
        reqline.advance(5);
        method = Method::POST;
    } else {
        let len = reqline
            .iter()
            .position(|&e| e == b' ')
            .ok_or(P::InvalidSeparator)?;
        method = Method::from_bytes(&reqline[..len]).ok_or(P::UnknownMethod)?;
        reqline.advance(len + 1);
    }

    const VER: &[u8; 9] = b" HTTP/1.1";
    let Some(VER) = reqline.last_chunk() else {
        return Ready(Err(P::UnsupportedVersion.into()));
    };
    reqline.truncate(reqline.len() - VER.len());
    let target = reqline;

    // ===== Headers =====

    let mut host = None;
    let mut content = None;

    for &header in unsafe { headers[..headers_len as usize].assume_init_ref() } {
        const BASIS: u32 = 0x811C_9DC5;
        const PRIME: u32 = 0x0100_0193;

        let mut line_buf = bytes.split_to(header as usize);
        line_buf.truncate(line_buf.len() - 2);
        let mut line = line_buf.as_mut_slice();
        let mut hash = BASIS;

        loop {
            let Some(byte_mut) = line.first_mut() else {
                return Ready(Err(P::InvalidSeparator.into()));
            };
            let byte = matches::HEADER_NAME[*byte_mut as usize];
            // Any invalid character will have it MSB set
            if byte & 128 == 0 {
                *byte_mut = byte;
                hash = PRIME.wrapping_mul(hash ^ byte as u32);
                line = &mut line[1..];
            } else {
                break
            }
        }

        let Some(b": ") = line.first_chunk() else {
            return Ready(Err(crate::headers::error::HeaderError::Invalid.into()));
        };

        let name_len = unsafe { line.as_ptr().offset_from_unsigned(line_buf.as_ptr()) };
        let hash = hash;
        let value;

        let name = match hash {
            hash::HOST => {
                line_buf.advance(name_len + 2);
                value = line_buf.freeze();
                if host.is_some() {
                    return Ready(Err(E::InvalidRepresentation));
                }
                host = Some(Host::from_bytes(value.clone())?);
                standard::HOST
            }
            hash::CONTENT_LENGTH => {
                line_buf.advance(name_len + 2);
                value = line_buf.freeze();
                if content.is_some() {
                    return Ready(Err(E::InvalidRepresentation));
                }
                if value.len() > 16 {
                    return Ready(Err(E::InvalidRepresentation));
                }
                let Some(content_len) = wrapping_atou(&value) else {
                    return Ready(Err(E::InvalidRepresentation));
                };
                content = Some(ContentKind::ContentLength(content_len));
                standard::CONTENT_LENGTH
            }
            hash::TRANSFER_ENCODING => {
                line_buf.advance(name_len + 2);
                value = line_buf.freeze();
                if content.is_some() {
                    return Ready(Err(E::InvalidRepresentation));
                }
                // TODO: support compressed transfer-encodings
                if &value != b"chunked" {
                    return Ready(Err(E::UnsupportedCodings));
                }
                content = Some(ContentKind::Chunked);
                standard::TRANSFER_ENCODING
            }
            hash::CONNECTION => {
                line_buf.advance(name_len + 2);
                value = line_buf.freeze();
                session.keep_alive = match value.as_slice() {
                    b"keep-alive" => true,
                    b"close" => false,
                    _ => return Ready(Err(E::InvalidConnectionOption)),
                };
                standard::CONNECTION
            }
            _ => {
                let name = line_buf.split_to(name_len);
                line_buf.advance(2);
                value = line_buf.freeze();
                // SAFETY: checks previously also ensure valid ASCII
                unsafe { HeaderName::from_bytes_unchecked(name.freeze()) }
            }
        };

        let value = HeaderValue::from_bytes(value)?;
        let field = HeaderField::with_hash(name, value, hash);
        let _ = session.headers.try_append_field(field);
    }

    // ===== Target URI =====

    let Some(host) = host else {
        return Ready(Err(E::InvalidHost));
    };
    let path = match target.as_slice() {
        // origin
        b"/" => Path::from_static(b"/"),
        // asterisk
        b"*" => Path::from_static(b"*"),
        // origin
        [b'/', ..] => Path::from_bytes(target)?,
        _ => if method != Method::CONNECT {
            // absolute
            let uri = HttpUri::from_bytes(target)?;
            if uri.host() == host.as_str() {
                return Ready(Err(P::MissmatchHost.into()));
            }
            uri.into()
        } else {
            // auth
            if target.as_slice() != host.as_str().as_bytes() {
                return Ready(Err(P::MissmatchHost.into()));
            }
            Path::from_static(b"")
        }
    };
    let target = HttpUri::from_parts(session.scheme, host, path);

    // ===== Message Body =====

    let content_kind = match content {
        Some(ok) => ok,
        None => ContentKind::ContentLength(0),
    };
    let decoder = BodyDecoder::new(content_kind);


    Ready(Ok(RequestContext {
        method,
        target,
        decoder,
    }))
}

// ===== Service Manager =====

pub struct RequestContext {
    pub method: Method,
    pub target: HttpUri,
    pub decoder: BodyDecoder,
}

impl RequestContext {
    pub fn build_request(
        &mut self,
        session: &mut Session,
        read_buffer: &mut BytesMut,
        cx: &mut std::task::Context,
    ) -> Request<Incoming> {
        let parts = request::Parts {
            method: self.method,
            uri: self.target.clone(),
            version: crate::http::Version::HTTP_11,
            headers: mem::take(&mut session.headers),
            extensions: crate::http::Extensions::new(),
        };
        let body = self.decoder.build_body(read_buffer, &mut session.shared, cx);
        Request::from_parts(parts, body)
    }

    /// Poll for request body, returns `true` if more read is required.
    ///
    /// This should be polled with the `Service` future.
    pub fn poll_read(
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

    pub fn build_response_writer<B>(
        &self,
        response: Response<B>,
        session: &mut Session,
        write_buffer: &mut BytesMut,
    ) -> (B, BodyEncoder)
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

        // https://www-rfc-editor.org/rfc/rfc9110.html#section-6.4.2-4
        if matches!(self.method, Method::HEAD) {
            return (body, BodyEncoder::new_length(0))
        }

        match clen {
            Some(len) => (body, BodyEncoder::new_length(len)),
            None => (body, BodyEncoder::new_chunked()),
        }
    }

    /// Returns `Ok(bool)` indicating whether message body draining is required.
    ///
    /// # Errors
    ///
    /// Returns error if message body draining is unable to be performed.
    pub fn needs_drain(&self) -> Result<bool, UserError> {
        self.decoder.needs_drain()
    }

    pub fn poll_drain(&mut self, read: usize) -> Poll<()> {
        self.decoder.poll_drain(read)
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

// ===== hash =====

mod hash {
    use crate::matches;

    pub const HOST: u32 = matches::hash_32(b"host");
    pub const CONTENT_LENGTH: u32 = matches::hash_32(b"content-length");
    pub const TRANSFER_ENCODING: u32 = matches::hash_32(b"transfer-encoding");
    pub const CONNECTION: u32 = matches::hash_32(b"connection");

    // pub const USER_AGENT: u32 = matches::hash_32(b"user-agent");
    // pub const TE: u32 = matches::hash_32(b"te");
    // pub const DATE: u32 = matches::hash_32(b"date");
    // pub const UPGRADE: u32 = matches::hash_32(b"date");
}

