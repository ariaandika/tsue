use std::task::Poll;
use tcio::{bytes::Bytes, ByteStr};

use crate::parser::simd::not_ascii_block;
use super::{
    path::Path,
    error::InvalidUri,
    scheme::Scheme,
};

// ```not_rust
// URI-reference = <URI-reference, see [URI], Section 4.1>
// absolute-URI  = <absolute-URI, see [URI], Section 4.3>
// relative-part = <relative-part, see [URI], Section 4.2>
// authority     = <authority, see [URI], Section 3.2>
// uri-host      = <host, see [URI], Section 3.2.2>
// port          = <port, see [URI], Section 3.2.3>
// path-abempty  = <path-abempty, see [URI], Section 3.3>
// segment       = <segment, see [URI], Section 3.3>
// query         = <query, see [URI], Section 3.4>
//
// absolute-path = 1*( "/" segment )
// partial-URI   = relative-part [ "?" query ]
// ```
//
// ```not_rust
// hier-part     = "//" authority path-abempty
//               / path-absolute
//               / path-rootless
//               / path-empty
// scheme        = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
// absolute-URI  = scheme ":" hier-part [ "?" query ]
// ```
//
// [request target]: <https://datatracker.ietf.org/doc/html/rfc9112#name-request-target>
// [uri reference]: <https://datatracker.ietf.org/doc/html/rfc9110#name-uri-references>

#[derive(Debug)]
pub enum Target {
    /// origin-form    = absolute-path [ "?" query ]
    Origin(Path),
    /// absolute-form  = absolute-URI
    Absolute(Bytes),
    /// authority-form = uri-host ":" port
    Authority(Bytes),
    /// asterisk-form  = "*"
    Asterisk,
}

impl Target {
    pub fn parse(mut bytes: Bytes) -> Poll<Result<Self, InvalidUri>> {
        // ===== Optimistic Case =====

        match bytes.split_first() {
            None => return Poll::Ready(Err(InvalidUri::Incomplete)),
            Some((b'*', b"")) => return Poll::Ready(Ok(Self::Asterisk)),
            Some((b'/', b"")) => return Poll::Ready(Ok(Self::Origin(Path::slash()))),
            Some((_, b"")) => return Poll::Ready(Err(InvalidUri::Incomplete)),
            Some((b'/', _)) => return Poll::Ready(Path::parse(bytes).map(Self::Origin)),
            _ => {}
        }

        let mut cursor = bytes.cursor_mut();

        'leader: {
            use crate::parser::simd::{BLOCK, MSB, LSB, COLON, HASH};

            while let Some(chunk) = cursor.peek_chunk::<BLOCK>() {
                let value = usize::from_ne_bytes(*chunk);

                // look for "#"
                let hash_xor = value ^ HASH;
                let hash_result = hash_xor.wrapping_sub(LSB) & !hash_xor;

                // look for ":"
                let lf_xor = value ^ COLON;
                let lf_result = lf_xor.wrapping_sub(LSB) & !lf_xor;

                let result = (hash_result | lf_result) & MSB;
                if result != 0 {
                    cursor.advance((result.trailing_zeros() / 8) as usize);
                    break 'leader;
                }

                if not_ascii_block!(value) {
                    return Poll::Ready(Err(InvalidUri::NonAscii));
                }

                cursor.advance(BLOCK);
            }

            while let Some(b) = cursor.next() {
                if b == b':' {
                    break 'leader;
                } else if !b.is_ascii() {
                    return Poll::Ready(Err(InvalidUri::NonAscii));
                }
            }

            return Poll::Pending;
        };

        // SAFETY: in leader iteration also check for valid ASCII
        let leader = unsafe { ByteStr::from_utf8_unchecked(cursor.split_to()) };

        // absolute-form
        if let Some(b"://") = cursor.peek_chunk() {
            let scheme = Scheme::new(leader);
            cursor.advance(3);
            cursor.advance_buf();
            todo!()

        } else {
            match cursor.peek() {
                // Some(b'/') => {
                //     let host = leader;
                //     todo!()
                //     // Self::Authority(host)
                // },

                // authority with port
                Some(b':') => {
                    let domain = leader;
                    cursor.advance(1);
                    cursor.advance_buf();

                },

                // authority without port
                None => return Poll::Ready(Err(InvalidUri::Incomplete)),
                Some(_) => unreachable!("invalid leader search"),
            }
        };

        todo!()
    }
}

