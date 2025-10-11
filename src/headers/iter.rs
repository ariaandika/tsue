use super::{HeaderMap, HeaderName, HeaderValue, entry::GetAll};

impl<'a> IntoIterator for &'a HeaderMap {
    type Item = <Iter<'a> as Iterator>::Item;

    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator returned from [`HeaderMap::iter`].
#[derive(Debug)]
pub struct Iter<'a> {
    map: &'a HeaderMap,
    n: usize,
    name: &'a HeaderName,
    iter: GetAll<'a>,
}

static PLACEHOLDER: HeaderName = HeaderName::placeholder();

impl<'a> Iter<'a> {
    pub(crate) fn new(map: &'a HeaderMap) -> Self {
        match map.entries().first() {
            Some(entry) => Self {
                map,
                n: 0,
                name: entry.name(),
                iter: GetAll::new(entry),
            },
            None => Self::empty(map),
        }
    }

    fn empty(map: &'a HeaderMap) -> Self {
        Self {
            map,
            n: 0,
            name: &PLACEHOLDER,
            iter: GetAll::empty(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a HeaderName, &'a HeaderValue);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(value) => return Some((self.name, value)),
                None => {
                    self.n += 1;
                    let entry = self.map.entries().get(self.n)?;
                    self.name = entry.name();
                    self.iter = self.map.get_all(entry.name());
                }
            }
        }
    }
}
