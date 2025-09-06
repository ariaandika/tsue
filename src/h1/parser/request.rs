use std::task::Poll;
use tcio::bytes::{Buf, BytesMut};

use super::{
    Target,
    error::{Error, ErrorKind},
    simd,
};
use crate::http::{Method, Version};

macro_rules! ready {
    ($e:expr) => {
        match $e {
            Some(ok) => ok,
            None => return Poll::Pending
        }
    };
}

macro_rules! err {
    ($variant:ident) => {
        Poll::Ready(Err(Error::from(ErrorKind::$variant)))
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
    pub fn matches(bytes: &mut BytesMut) -> Poll<Result<Reqline, Error>> {
        match_reqline(bytes)
    }
}

// ===== Parsing Request Line =====

fn match_reqline(bytes: &mut BytesMut) -> Poll<Result<Reqline, Error>> {
    let mut cursor = bytes.cursor_mut();

    let (method, offset) = {
        simd::byte_map! {
            const PAT =
                #[default(true)]
                #[false](b'!'..=b'~')
        }

        loop {
            let byte = ready!(cursor.next());
            if PAT[byte as usize] {
                if byte == b' ' {
                    break
                } else {
                    return err!(InvalidChar);
                }
            }
        }

        let method = cursor.advanced_slice().split_last().unwrap().1;

        match Method::from_bytes(method) {
            Some(ok) => (ok, cursor.steps()),
            _ => return err!(UnknownMethod),
        }
    };

    simd::match_target! {
        cursor;
        |val| match val {
            b' ' => { },
            _ => return err!(InvalidChar),
        };
        else {
            return Poll::Pending
        }
    }

    let version = match Version::from_bytes(ready!(cursor.next_chunk::<VERSION_SIZE>())) {
        Some(ok) => ok,
        None => return err!(UnsupportedVersion),
    };

    let tail = match ready!(cursor.next()) {
        b'\r' => match ready!(cursor.next()) {
            b'\n' => VERSION_SIZE + 3,
            _ => return err!(InvalidSeparator),
        },
        b'\n' => VERSION_SIZE + 2,
        _ => return err!(InvalidSeparator),
    };

    let len = cursor.steps();
    let mut target = bytes.split_to(len);
    target.advance(offset);
    target.truncate_off(tail);

    let target = Target::new(&method, target);

    Poll::Ready(Ok(Reqline {
        method,
        target,
        version,
    }))
}
