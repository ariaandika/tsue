
pub trait Zip<S> {
    type Output;

    fn zip(self, inner: S) -> Self::Output;
}

