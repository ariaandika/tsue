use tcio::{ByteStr, bytes::Cursor};

use super::{authority::Authority, error::InvalidUri, path::Path, scheme::Scheme, simd};

/// Request Target.
#[derive(Debug)]
pub enum Target {
    /// `/users/all?page=4&filter=available`
    Origin(Path),
    /// `http://example.com/users/all?page=4&filter=available`
    Absolute {
        scheme: Scheme,
        authority: Authority,
        path: Path,
    },
    /// `example.com:443`
    Authority(Authority),
    /// `*`
    Asterisk,
}

/// Parse full uri.
pub fn parse(string: ByteStr) -> Result<Target, InvalidUri> {
    match string.as_bytes() {
        [] => return Err(InvalidUri::Incomplete),
        [b'*'] => return Ok(Target::Asterisk),
        [b'/'] => return Ok(Target::Origin(Path::slash())),
        [b'/' | b'?', ..] => return Path::parse(string).map(Target::Origin),
        _ => {}
    }

    // absolute-form or authority-form

    let mut bytes = string.into_bytes();
    let mut cursor = bytes.cursor_mut();

    simd::match_uri_leader(&mut cursor);

    if let Some(b"://") = cursor.peek_chunk() {
        // ===== absolute-form =====

        // SAFETY: input is valid ASCII
        let scheme = unsafe { Scheme::new_unchecked(cursor.split_to()) };

        cursor.advance(b"://".len());
        cursor.advance_buf();

        simd::match_uri_leader(&mut cursor);

        let host = cursor.split_to();

        let Some(b':') = cursor.next() else {
            return Err(InvalidUri::Char);
        };
        cursor.advance_buf();

        let port = match_port(&mut cursor);
        let Some(b'/') = cursor.peek() else {
            return Err(InvalidUri::Char);
        };
        cursor.advance_buf();

        // SAFETY: input is valid ASCII
        let path = unsafe { ByteStr::from_utf8_unchecked(bytes) };

        match Path::parse(path) {
            Ok(path) => Ok(Target::Absolute {
                scheme,
                path,
                authority: unsafe { Authority::new_unchecked(host, port) },
            }),
            Err(err) => Err(err),
        }
    } else if let Some(b':') = cursor.peek() {
        // ===== authority-form =====

        let host = cursor.split_to();
        cursor.advance(1);

        let Some(port) = tcio::atou(cursor.as_slice()).and_then(|e| e.try_into().ok()) else {
            return Err(InvalidUri::Char);
        };

        // SAFETY: input is valid ASCII
        Ok(Target::Authority(unsafe {
            Authority::new_unchecked(host, port)
        }))
    } else {
        Err(InvalidUri::Char)
    }
}

fn match_port(cursor: &mut Cursor) -> u16 {
    debug_assert_eq!(cursor.steps(), 0);

    let mut port = 0u16;

    loop {
        let digit = match cursor.next() {
            Some(b'/') => {
                cursor.step_back(1);
                break;
            }
            // port more than 5 digit
            Some(_) if cursor.steps() > 5 => break,
            Some(digit) if digit.is_ascii_digit() => digit,
            _ => break,
        };

        unsafe {
            port = port
                .unchecked_mul(10)
                .unchecked_add(digit.unchecked_sub(b'0') as u16);
        }
    }

    port
}
