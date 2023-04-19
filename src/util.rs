use core::iter::Peekable;

pub trait IteratorExt: Iterator + Sized {
    #[inline]
    fn ensure_one(self, value: Self::Item) -> EnsureOne<Self> {
        EnsureOne {
            iter: self,
            value: Some(value),
        }
    }

    #[inline]
    fn delimited(self, value: Self::Item) -> Delimited<Self> {
        Delimited {
            separator: value,
            iter: self.peekable(),
            needs_separator: false,
        }
    }
}

impl<I: Iterator> IteratorExt for I {}

pub trait ChunksExt {
    fn chunks<const N: usize>(&self) -> Chunks<'_, N>;
}

impl ChunksExt for str {
    #[inline]
    fn chunks<const N: usize>(&self) -> Chunks<'_, N> {
        Chunks(self)
    }
}

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

pub struct Delimited<I: Iterator> {
    separator: I::Item,
    iter: Peekable<I>,
    needs_separator: bool,
}

impl<I: Iterator> Iterator for Delimited<I>
where
    I::Item: Clone,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.needs_separator && self.iter.peek().is_some() {
            self.needs_separator = false;
            Some(self.separator.clone())
        } else {
            self.needs_separator = true;
            self.iter.next()
        }
    }
}

pub struct Chunks<'str, const N: usize>(&'str str);

impl<'str, const N: usize> Iterator for Chunks<'str, N> {
    type Item = &'str str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        let (chunk, remaining) = self.0.split_at(self.0.len().min(N));
        self.0 = remaining;
        Some(chunk)
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
