use super::{IntoResponse, IntoResponseParts, Response};

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

