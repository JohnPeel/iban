pub trait IteratorExt: Iterator + Sized {
    fn ensure_one(self, value: Self::Item) -> EnsureOne<Self> {
        EnsureOne {
            iter: self,
            value: Some(value),
        }
    }
}

impl<I: Iterator> IteratorExt for I {}

pub struct EnsureOne<I: Iterator> {
    iter: I,
    value: Option<I::Item>,
}

impl<I: Iterator> Iterator for EnsureOne<I> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.iter.next() {
            self.value.take();
            return Some(item);
        }

        self.value.take()
    }
}
