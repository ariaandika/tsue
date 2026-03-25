use std::mem;
use std::task::Poll::{self, *};
use tcio::bytes::{Buf, BytesMut};
use tcio::num::{itoa, wrapping_atou};

use crate::body::{Body, Incoming};
use crate::h1::body::{BodyDecoder, BodyEncoder, ContentKind};
use crate::h1::states::Session;
use crate::headers::{HeaderField, HeaderName, HeaderValue, lookup};
use crate::http::{Method, Request, Response, httpdate_now, request, response};
use crate::matches;
use crate::proto::error::{ParseError, ProtoError, UserError};
use crate::uri::{Host, HttpUri, Path};

use ParseError as P;
use ProtoError as E;

pub fn poll_request(
    session: &mut Session,
    read_buffer: &mut BytesMut,
) -> Poll<Result<(request::Parts, RequestContext), ProtoError>> {
    // ===== Poll Reqline =====

    const MIN_REQLINE_LEN: usize = b"GET / HTTP/1.1".len();

    if read_buffer.len() < MIN_REQLINE_LEN {
        return Pending;
    }
    let Some(line) = matches::find_byte::<b'\n'>(read_buffer) else {
        return Pending;
    };
    if line.len() < MIN_REQLINE_LEN {
        return Ready(Err(P::InvalidSeparator.into()));
    }

    let (method, method_len) = if &line[..4] == b"GET " {
        (Method::GET, 3)
    } else if &line[..5] == b"POST " {
        (Method::POST, 4)
    } else {
        let len = line
            .iter()
            .position(|&e| e == b' ')
            .ok_or(P::InvalidSeparator)?;
        let method = match &line[..len] {
            b"HEAD" => Method::HEAD,
            b"PUT" => Method::PUT,
            b"DELETE" => Method::DELETE,
            b"OPTIONS" => Method::OPTIONS,
            b"TRACE" => Method::TRACE,
            b"PATCH" => Method::PATCH,
            // An origin server MAY accept a CONNECT request, but most origin servers do not
            // implement CONNECT.
            // b"CONNECT" => unimplemented!(),
            _ => return Ready(Err(P::UnknownMethod.into())),
        };
        (method, len)
    };

    let Some(suffix) = line.last_chunk() else {
        return Ready(Err(P::InvalidSeparator.into()));
    };
    const SUFFIX: &[u8; 10] = b" HTTP/1.1\r";
    if suffix != SUFFIX {
        return Ready(Err(P::UnsupportedVersion.into()));
    }

    // SAFETY: `line` is a subset of `read_buffer`, `+1` because `line` will always exclude the
    // '\n' in the `read_buffer`
    let mut state = unsafe { read_buffer.get_unchecked(..line.len() + 1) };

    // ===== Poll Headers =====

    let mut headers = Headers::new();
    loop {
        if state.first_chunk::<2>() == Some(b"\r\n") {
            break;
        }
        let Some(line) = matches::find_byte::<b'\n'>(state) else {
            return Pending;
        };
        if line.is_empty() {
            return Ready(Err(P::InvalidSeparator.into()));
        }
        let line_len = line.len() + 1;
        headers.insert(line_len)?;
        state = &state[line_len..];
    }

    // polling complete, no more `Pending`

    let target = unsafe {
        // SAFETY: `line` is a subset of `read_buffer`, `+1` because `line` will always exclude the
        // '\n' in the `read_buffer`
        let mut reqline = read_buffer.split_to_unchecked(line.len() + 1);
        // remove reqline method
        // SAFETY: checked at the start of the function
        reqline.advance_unchecked(method_len + 1);
        // remove reqline version, `+1` the '\n'
        // SAFETY: the subtraction will not overflow, thus it always less than `reqline.len()`
        reqline.set_len(reqline.len() - (SUFFIX.len() + 1));
        reqline
    };

    // ===== Headers =====

    let mut host = None;
    let mut content = None;

    for &hdr_index in headers.as_slice() {
        // SAFETY: `hdr_index` is in bounds, see poll headers loop above
        let mut line = unsafe { read_buffer.split_to_unchecked(hdr_index as usize) };
        line.truncate(line.len() - 2);
        let mut line_ref = line.as_mut_slice();
        let mut hash = matches::BASIS_32;

        // look for ':' separator while hashing and validating
        loop {
            let Some((byte_mut, rest)) = line_ref.split_first_mut() else {
                // no ':' found
                return Ready(Err(P::InvalidSeparator.into()));
            };
            let byte = matches::HEADER_NAME[*byte_mut as usize];
            // Any invalid character will have it MSB set
            if byte & 128 == 0 {
                *byte_mut = byte;
                hash = matches::PRIME_32.wrapping_mul(hash ^ byte as u32);
                line_ref = rest;
            } else {
                if *byte_mut != b':' {
                    return Ready(Err(P::InvalidSeparator.into()));
                };
                line_ref = rest;
                break
            }
        }

        // SAFETY:
        // - `line_ref` is subset of `line`
        // - `-1` is the `:`
        let name_ref = unsafe {
            let name_len = line_ref.as_ptr().offset_from_unsigned(line.as_ptr()) - 1;
            line.get_unchecked(..name_len)
        };
        let name = if let Some(name) = lookup::request_header(hash, name_ref) {
            // using static header name, no need to split the Bytes
            // SAFETY: `line.len() >= name_ref.len()`
            unsafe { line.advance_unchecked(name_ref.len()) };
            name
        } else {
            // arbitrary header name, split the Bytes
            unsafe {
                // SAFETY: `line.len() >= name_ref.len()`
                let name = line.split_to_unchecked(name_ref.len());
                // SAFETY: checks in previous loop ensure valid header name
                HeaderName::from_bytes_unchecked(name.freeze())
            }
        };

        debug_assert_eq!(line.first(), Some(&b':'));
        unsafe { line.advance_unchecked(1) };

        // separator may contains whitespace
        while line.first() == Some(&b' ') {
            line.advance(1);
        }
        let value = line.freeze();

        const HOST: u32 = matches::hash_32(b"host");
        const CONTENT_LENGTH: u32 = matches::hash_32(b"content-length");
        const TRANSFER_ENCODING: u32 = matches::hash_32(b"transfer-encoding");
        const CONNECTION: u32 = matches::hash_32(b"connection");

        match hash {
            HOST => {
                if host.is_some() {
                    return Ready(Err(E::InvalidRepresentation));
                }
                host = Some(Host::from_bytes(value.clone())?);
            }
            CONTENT_LENGTH => {
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
            }
            TRANSFER_ENCODING => {
                if content.is_some() {
                    return Ready(Err(E::InvalidRepresentation));
                }
                // TODO: support compressed transfer-encodings
                if &value != b"chunked" {
                    return Ready(Err(E::UnsupportedCodings));
                }
                content = Some(ContentKind::Chunked);
            }
            CONNECTION => {
                session.keep_alive = match value.as_slice() {
                    b"keep-alive" => true,
                    b"close" => false,
                    _ => return Ready(Err(E::InvalidConnectionOption)),
                };
            }
            _ => {}
        };

        let value = HeaderValue::from_bytes(value)?;
        let field = HeaderField::with_hash(name, value, hash);
        let _ = session.headers.try_append_field(field);
    }

    debug_assert_eq!(read_buffer.first_chunk(), Some(b"\r\n"));
    unsafe { read_buffer.advance_unchecked(2) };

    // ===== Target URI =====

    let Some(host) = host else {
        return Ready(Err(E::InvalidHost));
    };
    // - authority form are prohibited
    // - asterisk form handled separately with OPTIONS method
    let uri = if target.first() == Some(&b'/') {
        // origin-form
        let path = Path::from_bytes(target)?;
        HttpUri::from_parts(session.scheme, host, path)
    } else {
        // absolute-form
        let uri = HttpUri::from_bytes(target)?;
        if uri.host() == host.as_str() {
            return Ready(Err(P::MissmatchHost.into()));
        }
        uri
    };

    // ===== Message Body =====

    let content_kind = match content {
        Some(ok) => ok,
        None => ContentKind::ContentLength(0),
    };
    let decoder = BodyDecoder::new(content_kind);

    // ===== Request =====

    let parts = request::Parts {
        method,
        uri,
        version: crate::http::Version::HTTP_11,
        headers: mem::take(&mut session.headers),
    };

    let context = RequestContext {
        method,
        decoder,
    };

    Ready(Ok((parts, context)))
}

// ===== Headers Index =====

struct Headers {
    headers: [mem::MaybeUninit<u16>; Self::MAX_HEADERS as usize],
    len: u8,
}

impl Headers {
    const MAX_HEADERS: u8 = 32;

    fn new() -> Self {
        Self {
            headers: [mem::MaybeUninit::uninit();_],
            len: 0,
        }
    }

    fn insert(&mut self, line_len: usize) -> Result<(), ProtoError> {
        if self.len >= Self::MAX_HEADERS {
            return Err(E::TooManyHeaders);
        }
        let Ok(value) = u16::try_from(line_len) else {
            return Err(E::ParseError(P::TooLong));
        };
        self.headers[self.len as usize].write(value);
        self.len += 1;
        Ok(())
    }

    fn as_slice(&self) -> &[u16] {
        unsafe {
            self.headers
                .get_unchecked(..self.len as usize)
                .assume_init_ref()
        }
    }
}

// ===== Service Manager =====

pub struct RequestContext {
    pub method: Method,
    pub decoder: BodyDecoder,
}

impl RequestContext {
    pub fn build_request(
        &mut self,
        parts: request::Parts,
        session: &mut Session,
        read_buffer: &mut BytesMut,
        cx: &mut std::task::Context,
    ) -> Request<Incoming> {
        Request::from_parts(
            parts,
            self.decoder
                .build_body(read_buffer, &mut session.shared, cx),
        )
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
