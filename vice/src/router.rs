use crate::http::{Request, Response};
use hyper::service::Service;
use std::{future::Future, sync::Arc};


pub struct Router {
    inner: Arc<
        Box<
            dyn Service<
                Request,
                Response = Response,
                Error = Box<dyn std::error::Error>,
                Future = dyn Future<Output = Result<Response, Box<dyn std::error::Error>>>,
            >,
        >,
    >,
}

impl Router {
    pub fn route<S>(mut self, service: S) where S: Service<Request> {
        let _service = Box::new(service);
        let _app = Arc::get_mut(&mut self.inner).expect("should not be cloned in route builder");
        // LATEST: vice
    }
}

