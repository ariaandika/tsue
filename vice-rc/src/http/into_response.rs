//! the [`IntoResponse`] and [`IntoResponseParts`] trait
use super::{response, Response};

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
    fn into_response_parts(self, parts: response::Parts) -> response::Parts;
}


//
// NOTE: impls
//

macro_rules! into_response {
    ($target:ty,$self:ident => $body:expr) => {
        impl IntoResponse for $target {
            fn into_response($self) -> Response {
                $body
            }
        }
    };
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

/*

into_response!(io::Error, self => {
    log::error!("{self}");
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(<_>::from("Internal Server Error".as_bytes()))
        .unwrap()
});

*/
