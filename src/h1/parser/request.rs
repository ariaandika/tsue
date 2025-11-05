use std::{slice::from_raw_parts, task::Poll};
use tcio::bytes::BytesMut;

use super::{
    Target,
    error::{HttpError, ErrorKind},
    matches,
};
use crate::http::{Method, Version};

macro_rules! err {
    ($variant:ident) => {
        Poll::Ready(Err(HttpError::from(ErrorKind::$variant)))
    };
}

const VERSION_SIZE: usize = b"HTTP/1.1".len();

#[derive(Debug)]
pub struct Reqline {
    pub method: Method,
    pub target: Target,
    pub version: Version,
}

impl Reqline {
    #[inline]
    pub fn parse_chunk(bytes: &mut BytesMut) -> Poll<Result<Reqline, HttpError>> {
        parse_chunk_reqline(bytes)
    }
}

// ===== Request Line =====

fn parse_chunk_reqline(bytes: &mut BytesMut) -> Poll<Result<Reqline, HttpError>> {
    let mut reqline = {
        let mut state = bytes.as_slice();

        let delim = matches::split_crlf!(state else {
            return Poll::Pending
        });

        let crlf = match delim {
            b'\r' => match state.split_first() {
                Some((b'\n', rest)) => {
                    state = rest;
                    2
                },
                Some(_) => return err!(InvalidSeparator),
                None => return Poll::Pending,
            },
            b'\n' => 1,
            _ => return err!(InvalidSeparator),
        };

        let mut reqline = bytes.split_to_ptr(state.as_ptr());
        reqline.truncate_off(crlf);
        reqline
    };

    let mut target = reqline.as_slice();

    let method = {
        let mut state = target;

        while let [byte, rest @ ..] = state {
            if !matches::is_method(*byte) {
                if *byte == b' ' {
                    target = rest;
                    break;
                } else {
                    return err!(InvalidMethod);
                }
            }
            state = rest;
        }

        let method = unsafe {
            let start = reqline.as_ptr();
            let len = state.as_ptr().offset_from_unsigned(start);
            from_raw_parts(start, len)
        };

        match Method::from_bytes(method) {
            Some(ok) => {
                ok
            },
            _ => return err!(UnknownMethod),
        }
    };

    let version = {
        let Some(([rest @ .., b' '], version)) = target.split_last_chunk::<VERSION_SIZE>() else {
            return err!(InvalidSeparator)
        };

        target = rest;

        match Version::from_bytes(version) {
            Some(ok) => ok,
            None => return err!(UnsupportedVersion),
        }
    };

    // SAFETY: `target` is only sliced within `target` bounds itself
    unsafe {
        let len = target.len();
        reqline.advance_to_ptr(target.as_ptr());
        reqline.truncate(len);
    }
    let target = Target::new(&method, reqline);

    Poll::Ready(Ok(Reqline {
        method,
        target,
        version,
    }))
}
