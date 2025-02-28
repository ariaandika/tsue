use super::Service;

pub fn service_fn<F>(f: F) -> ServiceFn<F> {
    ServiceFn { f }
}

#[derive(Clone)]
pub struct ServiceFn<F> {
    f: F
}

impl<Request,Response,Error,F,Fut> Service<Request> for ServiceFn<F>
where
    F: Fn(Request) -> Fut,
    Fut: Future<Output = Result<Response,Error>>
{
    type Response = Response;
    type Error = Error;
    type Future = Fut;

    fn call(&self, request: Request) -> Self::Future {
        (self.f)(request)
    }
}

