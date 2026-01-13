use super::{HeaderField, HeaderMap, HeaderName, HeaderValue, field::GetAll};

impl<'a> IntoIterator for &'a HeaderMap {
    type Item = <Iter<'a> as Iterator>::Item;

    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug)]
pub struct Iter<'a> {
    iter: std::slice::Iter<'a, HeaderField>,
    current: Option<(&'a HeaderName, GetAll<'a>)>,
}

impl<'a> Iter<'a> {
    pub(crate) fn new(map: &'a HeaderMap) -> Self {
        let mut iter = map.fields().iter();
        Self {
            current: iter.next().map(|e| (e.name(), e.iter())),
            iter,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a HeaderName, &'a HeaderValue);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((name, values)) = &mut self.current
                && let Some(value) = values.next()
            {
                return Some((name, value));
            }

            let field = self.iter.next()?;
            self.current = Some((field.name(), field.iter()));
        }
    }
}
