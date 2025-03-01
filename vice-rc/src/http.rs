//! http protocol
#![allow(dead_code)]
use crate::{
    body::{Body, ResBody},
    bytestring::ByteStr,
};
use bytes::Bytes;

pub mod parse;
pub mod service;

pub const MAX_HEADER: usize = 32;

#[derive(Default)]
pub enum Method {
    #[default]
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
    CONNECT,
}

#[derive(Default)]
pub enum Version {
    Http10,
    #[default]
    Http11,
    Http2,
}

impl Version {
    pub fn into_bytes(self) -> Bytes {
        match self {
            Version::Http10 => Bytes::from_static(b"HTTP/1.0"),
            Version::Http11 => Bytes::from_static(b"HTTP/1.1"),
            Version::Http2 => Bytes::from_static(b"HTTP/2"),
        }
    }
}

#[derive(Clone, Default)]
pub struct Header {
    pub name: ByteStr,
    pub value: Bytes,
}

impl Header {
    const fn new() -> Header {
        Header {
            name: ByteStr::new(),
            value: Bytes::new(),
        }
    }
}

#[derive(Default)]
pub struct ReqParts {
    method: Method,
    path: ByteStr,
    version: Version,
    headers: [Header;MAX_HEADER],
    header_len: usize,
}

impl ReqParts {
    pub fn headers(&self) -> &[Header] {
        &self.headers[..self.header_len]
    }
}

#[derive(Default)]
pub struct Request {
    parts: ReqParts,
    body: Body,
}

impl Request {
    fn into_parts(self) -> (ReqParts,Body) {
        (self.parts,self.body)
    }

    fn into_body(self) -> Body {
        self.body
    }
}

#[derive(Default)]
pub struct ResParts {
    version: Version,
    status: Bytes,
    reason: Bytes,
    headers: [Header;MAX_HEADER],
}

#[derive(Default)]
pub struct Response {
    parts: ResParts,
    body: ResBody,
}

/// a type that can be constructed by request
///
/// this trait is used as request handler parameters
pub trait FromRequest: Sized {
    type Error;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request(req: Request) -> Self::Future;
}

/// a type that can be constructed by request parts
///
/// this trait is used as request handler parameters
pub trait FromRequestParts: Sized {
    type Error;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request_parts(parts: &mut ReqParts) -> Self::Future;
}

/// a type that can be converted into response
///
/// this trait is used as request handler return type
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

/// a type that can be converted into response parts
///
/// this trait is used as request handler return type
pub trait IntoResponseParts {
    fn into_response_parts(self, parts: ResParts) -> ResParts;
}

mod impls {
    #![allow(dead_code,unused_imports)]

    use super::*;
    use bytes::{Bytes, BytesMut};
    use std::{
        convert::Infallible,
        future::{ready, Ready},
        io,
        pin::Pin,
    };

    macro_rules! from_request {
        ($self:ty, $($id:ident = $t:ty;)* ($req:pat) => $body: expr) => {
            impl FromRequest for $self {
                $(type $id = $t;)*

                fn from_request($req: Request) -> Self::Future {
                    $body
                }
            }
        };
    }

    // macro_rules! into_response {
    //     ($target:ty,$self:ident => $body:expr) => {
    //         impl IntoResponse for $target {
    //             fn into_response($self) -> Response {
    //                 $body
    //             }
    //         }
    //     };
    // }

    impl<F> FromRequest for F
    where
        F: FromRequestParts
    {
        type Error = <F as FromRequestParts>::Error;
        type Future = <F as FromRequestParts>::Future;

        fn from_request(req: Request) -> Self::Future {
            Self::from_request_parts(&mut req.into_parts().0)
        }
    }

    impl FromRequestParts for () {
        type Error = Infallible;
        type Future = Ready<Result<Self, Infallible>>;

        fn from_request_parts(_: &mut ReqParts) -> Self::Future {
            ready(Ok(()))
        }
    }

    impl<T,E> IntoResponse for Result<T,E>
    where
        T: IntoResponse,
        E: IntoResponse,
    {
        fn into_response(self) -> Response {
            match self {
                Ok(ok) => ok.into_response(),
                Err(err) => err.into_response(),
            }
        }
    }

    from_request! {
        Request,
        Error = Infallible;
        Future = Ready<Result<Self,Infallible>>;
        (req) => ready(Ok(req))
    }

    // NOTE:
    // using Pin<Box> in association type is worth it instead of impl Future,
    // because it can be referenced externally

    /*

    from_request! {
        BytesMut,
        Error = io::Error;
        Future = Pin<Box<dyn Future<Output = io::Result<Self>>>>;
        (req) => Box::pin(req.into_body().bytes_mut())
    }

    from_request! {
        Bytes,
        Error = io::Error;
        Future = future::MapOk<<BytesMut as FromRequest>::Future, fn(BytesMut) -> Bytes>;
        (req) => BytesMut::from_request(req).map_ok(BytesMut::freeze as _)
    }

    from_request! {
        Vec<u8>,
        Error = io::Error;
        Future = future::MapOk<<BytesMut as FromRequest>::Future, fn(BytesMut) -> Vec<u8>>;
        (req) => BytesMut::from_request(req).map_ok(Into::into as _)
    }

    from_request! {
        String,
        Error = BadRequest;
        Future = future::Map<
            <BytesMut as FromRequest>::Future,
            fn(io::Result<BytesMut>) -> Result<String, BadRequest>,
        >;
        (req) => BytesMut::from_request(req).map(|e|String::from_utf8(e?.into()).map_err(Into::into))
    }

    into_response!((), self => <_>::default());
    into_response!(Response, self => self);
    into_response!(String, self => Response::new(self.into()));
    into_response!(Infallible, self => match self { });
    into_response!(io::Error, self => {
        tracing::error!("{self}");
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(<_>::from("Internal Server Error".as_bytes()))
            .unwrap()
    });

    */
}

