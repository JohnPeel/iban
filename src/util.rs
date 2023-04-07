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

pub fn digits(mut value: u8) -> impl Iterator<Item = u8> {
    let hundreds = value / 100;
    value -= hundreds * 100;
    let tens = value / 10;
    value -= tens * 10;
    let ones = value;

    [hundreds, tens, ones]
        .into_iter()
        // Skip leading zeros
        .skip_while(|&b| b == 0)
        // Ensure at least one value (0) is provided by this iterator.
        .ensure_one(0)
}
