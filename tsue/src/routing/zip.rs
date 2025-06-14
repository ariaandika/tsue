/// 'Zips up' two services into single service.
pub trait Zip<S> {
    /// The `zipped` output service.
    type Output;

    fn zip(self, inner: S) -> Self::Output;
}

