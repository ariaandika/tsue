use std::task::Poll;
use tcio::bytes::BytesMut;

use super::{
    error::{Error, ErrorKind},
    simd,
};
use crate::{
    http::{Method, Version},
    parser::uri::{self, Uri},
};

macro_rules! err {
    ($variant:ident) => {
        Poll::Ready(Err(Error::from(ErrorKind::$variant)))
    };
}

const VERSION_SIZE: usize = b"HTTP/1.1".len();

#[derive(Debug)]
pub struct Reqline {
    pub method: Method,
    pub target: Uri,
    pub version: Version,
}

// ===== Parsing Request Line =====
//
// #1 SIMD Find CRLF, find Method, find backward Version
//
// - fast `Pending` case
// - cannot check for valid URI char in SIMD
//
// #2 Find Method, SIMD find URI, find Version
//
// - slightly slower `Pending` case
// - can check for valid URI char in SIMD
//
// #3 SIMD Find CRLF, find Method, find backward Version, parse URI (#CURRENT)
//
// - fast `Pending` case
// - parsing while checking valid URI char
// - merged logic code
//
// #4 Find Method, parse URI, find Version
//
// - slow `Pending` case
// - parsing while checking valid URI char
// - parsing URI also check for separator
// - merged logic code

pub fn parse_reqline(bytes: &mut BytesMut) -> Poll<Result<Reqline, Error>> {
    let mut cursor = bytes.cursor_mut();

    simd::match_crlf!(cursor);

    let crlf = match cursor.next().unwrap() {
        b'\n' => 1,
        b'\r' => match cursor.next() {
            Some(b'\n') => 2,
            Some(_) => return err!(InvalidSeparator),
            None => return Poll::Pending,
        },
        _ => return err!(InvalidSeparator),
    };

    let mut reqline = cursor.split_to();
    reqline.truncate_off(crlf);

    let method = {
        let mut cursor = reqline.cursor_mut();

        loop {
            match cursor.next() {
                Some(b' ') => break,
                Some(_) => {},
                None => return err!(InvalidSeparator),
            }
        }
        cursor.step_back(1);

        let Some(ok) = Method::from_bytes(cursor.advanced_slice()) else {
            return err!(UnknownMethod);
        };
        cursor.advance(1);
        cursor.advance_buf();

        ok
    };

    let version = {
        let Some((rest, version)) = reqline.split_last_chunk::<VERSION_SIZE>() else {
            return err!(UnsupportedVersion);
        };

        let Some(ok) = Version::from_bytes(version) else {
            return err!(UnsupportedVersion);
        };

        if !matches!(rest.last(), Some(b' ')) {
            return err!(InvalidSeparator);
        }

        reqline.truncate_off(VERSION_SIZE + 1);
        ok
    };

    match uri::parse(reqline.freeze()) {
        Ok(target) => Poll::Ready(Ok(Reqline {
            method,
            target,
            version,
        })),
        Err(err) => Poll::Ready(Err(err.into())),
    }
}

