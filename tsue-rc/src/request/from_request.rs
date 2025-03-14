use super::{FromRequest, FromRequestParts, Parts, Request};
use crate::http::Method;
use bytes::BytesMut;
use std::{
    convert::Infallible,
    future::{ready, Ready},
    io,
    pin::Pin,
};

// NOTE:
// using Pin<Box> in association type is worth it instead of impl Future,
// because it can be referenced externally

macro_rules! from_parts {
    ($self:ty, $($id:ident = $t:ty;)* ($parts:pat) => $body: expr) => {
        impl FromRequestParts for $self {
            $(type $id = $t;)*

            fn from_request_parts($parts: &mut Parts) -> Self::Future {
                $body
            }
        }
    };
}

from_parts! {
    Method,
    Error = Infallible;
    Future = Ready<Result<Self,Infallible>>;
    (parts) => ready(Ok(parts.method))
}

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

/// anything that implement `FromRequestParts` also implement `FromRequest`
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

from_request! {
    (),
    Error = Infallible;
    Future = Ready<Result<Self,Infallible>>;
    (_) => ready(Ok(()))
}

from_request! {
    Request,
    Error = Infallible;
    Future = Ready<Result<Self,Infallible>>;
    (req) => ready(Ok(req))
}

from_request! {
    BytesMut,
    Error = io::Error;
    Future = Pin<Box<dyn Future<Output = io::Result<Self>>>>;
    (req) => Box::pin(req.into_body().bytes_mut())
}

/*

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

*/

