use crate::service::HttpService;

/// 'Zips up' two services into single service.
pub trait Zip {
    /// The `zipped` output service.
    type Output<S: HttpService>: HttpService;

    fn zip<S: HttpService>(self, inner: S) -> Self::Output<S>;
}

