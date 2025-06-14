
// NOTE: limitation
// when user wants to compose with `Router<impl HttpService>`,
// router cant merge anymore because returned router does not implement zip
// anymore, but instead only `impl HttpService`

/// 'Zips up' two services into single service.
pub trait Zip<S> {
    /// The `zipped` output service.
    type Output;

    fn zip(self, inner: S) -> Self::Output;
}

