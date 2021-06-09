use super::pair::Pair;
use std::{
    fmt::{self, Debug, Formatter},
    ops::{Index, IndexMut},
};

pub struct Matrix<T> {
    dimensions: Pair<usize>,
    slice: Box<[T]>,
}

impl<T> Matrix<T> {
    pub fn new(dimensions: Pair<usize>, value: T) -> Self
    where
        T: Clone,
    {
        let slice = vec![value; dimensions.left * dimensions.right].into_boxed_slice();
        Self { dimensions, slice }
    }

    fn offset(&self, index: Pair<usize>) -> usize {
        assert!(index.zip(self.dimensions).into_iter().all(|(i, d)| i < d));
        index.left * self.dimensions.right + index.right
    }
}

impl<T: Debug> Debug for Matrix<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn write<I: Iterator, N>(mut items: I, f: &mut Formatter<'_>, next: N) -> fmt::Result
        where
            N: Fn(I::Item, &mut Formatter<'_>) -> fmt::Result,
        {
            write!(f, "[")?;
            if let Some(item) = items.next() {
                next(item, f)?;
                for item in items {
                    write!(f, ", ")?;
                    next(item, f)?;
                }
            }

            write!(f, "]")
        }

        let matrix = (0..self.dimensions.left)
            .map(|left| (0..self.dimensions.right).map(move |right| &self[Pair::new(left, right)]));

        write(matrix, f, |vector, f| {
            write(vector, f, |value, f| write!(f, "{:?}", value))
        })
    }
}

impl<T> Index<Pair<usize>> for Matrix<T> {
    type Output = T;

    fn index(&self, index: Pair<usize>) -> &Self::Output {
        &self.slice[self.offset(index)]
    }
}

impl<T> IndexMut<Pair<usize>> for Matrix<T> {
    fn index_mut(&mut self, index: Pair<usize>) -> &mut Self::Output {
        &mut self.slice[self.offset(index)]
    }
}
