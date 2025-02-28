//! contains http related protocols
use crate::body::{Body, ResBody};
use std::future::Future;

pub use http::{header, method, StatusCode};

pub type ReqParts = http::request::Parts;
pub type ResParts = http::response::Parts;
pub type Request<B = Body> = http::Request<B>;
pub type Response<B = ResBody> = http::Response<B>;

/// a type that can be constructed by request
///
/// this trait is used as request handler parameters
pub trait FromRequest: Sized {
    type Error: IntoResponse;
    type Future: Future<Output = Result<Self,Self::Error>>;
    fn from_request(req: Request) -> Self::Future;
}

/// a type that can be constructed by request parts
///
/// this trait is used as request handler parameters
pub trait FromRequestPart: Sized {
    type Error: IntoResponse;
    type Future: Future<Output = Result<Self,Self::Error>>;
    fn from_request_part(parts: &mut ReqParts) -> Self::Future;
}

/// a type that can be converted into response parts
///
/// this trait is used as request handler return type
pub trait IntoResponsePart {
    fn into_response_parts(self, parts: ResParts) -> Response;
}

/// a type that can be converted into response
///
/// this trait is used as request handler return type
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

mod impls {
    use super::*;
    use crate::error::BadRequest;
    use bytes::{Bytes, BytesMut};
    use futures_util::{future, FutureExt, TryFutureExt};
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

    macro_rules! into_response {
        ($target:ty,$self:ident => $body:expr) => {
            impl IntoResponse for $target {
                fn into_response($self) -> Response {
                    $body
                }
            }
        };
    }

    impl FromRequestPart for () {
        type Error = Infallible;
        type Future = Ready<Result<Self,Infallible>>;
        fn from_request_part(_: &mut ReqParts) -> Self::Future {
            ready(Ok(()))
        }
    }

    impl<F> FromRequest for F
    where
        F: FromRequestPart
    {
        type Error = <F as FromRequestPart>::Error;
        type Future = <F as FromRequestPart>::Future;

        fn from_request(req: Request) -> Self::Future {
            Self::from_request_part(&mut req.into_parts().0)
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

    impl<T,E> IntoResponse for Result<T,E>
    where
        T: IntoResponse,
        E: IntoResponse
    {
        fn into_response(self) -> Response {
            match self {
                Ok(ok) => ok.into_response(),
                Err(err) => err.into_response(),
            }
        }
    }
}

