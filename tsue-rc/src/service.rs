pub mod servicefn;

pub trait Service<Request> {
    type Response;
    type Error;
    type Future: Future<Output = Result<Self::Response,Self::Error>>;

    fn call(&self, request: Request) -> Self::Future;
}

