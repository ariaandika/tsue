use crate::http::StatusCode;
use super::{IntoResponse, IntoResponseParts, Parts, Response};

macro_rules! into_response {
    ($target:ty,$self:ident => $body:expr) => {
        impl IntoResponse for $target {
            fn into_response($self) -> Response {
                $body
            }
        }
    };
}

/// anything that implement `IntoResponseParts` also implement `IntoResponse`
impl<R> IntoResponse for R
where
    R: IntoResponseParts
{
    fn into_response(self) -> Response {
        let (mut parts,body) = Response::default().into_parts();
        parts = R::into_response_parts(self, parts);
        Response::from_parts(parts,body)
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

into_response!((), self => <_>::default());
into_response!(Response, self => self);
into_response!(String, self => Response::new(self.into()));
into_response!(std::convert::Infallible, self => match self { });

impl IntoResponseParts for StatusCode {
    fn into_response_parts(self, mut parts: Parts) -> Parts {
        parts.status = self;
        parts
    }
}

// impl<T> IntoResponseParts for (&'static str, T)
// where
//     T: Into<Bytes>,
// {
//     fn into_response_parts(self, mut parts: response::Parts) -> response::Parts {
//         if let Ok(value) = HeaderValue::from_maybe_shared(self.1.into()) {
//             parts.headers.insert(self.0, value);
//         }
//         parts
//     }
// }

macro_rules! into_response_tuple {
    (@$($r:ident,)*) => {
        impl<$($r,)*R> IntoResponse for ($($r,)*R)
        where
            $($r: IntoResponseParts,)*
            R: IntoResponse,
        {
            fn into_response(self) -> Response {
                #![allow(non_snake_case)]
                let ($($r,)*r) = self;
                let (mut parts,body) = r.into_response().into_parts();
                $(parts = $r.into_response_parts(parts);)*
                Response::from_parts(parts, body)
            }
        }
    };
    () => { };
    ($r:ident,$($r2:ident,)*) => {
        into_response_tuple!($($r2,)*);
        into_response_tuple!(@ $r, $($r2,)*);
    };
}

into_response_tuple!(R1,R2,R3,R4,R5,R6,R7,R8,);

