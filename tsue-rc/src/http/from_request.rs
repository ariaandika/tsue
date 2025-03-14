//! the [`FromRequest`] and [`FromRequestParts`] trait
use super::{request, Request};
use std::{
    convert::Infallible,
    future::{ready, Ready},
};

/// a type that can be constructed from request
///
/// this trait is used as request handler parameters
pub trait FromRequest: Sized {
    type Error;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request(req: Request) -> Self::Future;
}

/// a type that can be constructed from request parts
///
/// this trait is used as request handler parameters
pub trait FromRequestParts: Sized {
    type Error;
    type Future: Future<Output = Result<Self, Self::Error>>;
    fn from_request_parts(parts: &mut request::Parts) -> Self::Future;
}


//
// NOTE: impls
//

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

// NOTE:
// using Pin<Box> in association type is worth it instead of impl Future,
// because it can be referenced externally

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

    fn from_request_parts(_: &mut request::Parts) -> Self::Future {
        ready(Ok(()))
    }
}

from_request! {
    Request,
    Error = Infallible;
    Future = Ready<Result<Self,Infallible>>;
    (req) => ready(Ok(req))
}

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

*/
