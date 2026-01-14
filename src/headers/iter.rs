use super::{HeaderField, HeaderMap, HeaderName, HeaderValue, field::GetAll};

#[derive(Debug, Clone)]
pub struct Iter<'a> {
    iter: std::slice::Iter<'a, Option<HeaderField>>,
    current: Option<(&'a HeaderName, GetAll<'a>)>,
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

            let field = self.iter.find_map(|e| e.as_ref())?;
            self.current = Some((field.name(), field.iter()));
        }
    }
}

impl<'a> IntoIterator for &'a HeaderMap {
    type Item = <Iter<'a> as Iterator>::Item;

    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let mut iter = self.fields().iter();
        Iter {
            current: iter.find_map(|e| e.as_ref().map(|e| (e.name(), e.iter()))),
            iter,
        }
    }
}
