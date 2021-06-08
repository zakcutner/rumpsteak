use super::pair::Pair;
use std::{
    fmt::{self, Debug, Formatter},
    iter,
};

pub struct Matrix<T> {
    dimensions: Pair<usize>,
    slice: Box<[T]>,
}

impl<T> Matrix<T> {
    pub fn new(dimensions: Pair<usize>) -> Self
    where
        T: Default,
    {
        let slice = iter::repeat_with(Default::default);
        let slice = slice.take(dimensions.left * dimensions.right).collect();
        Self { dimensions, slice }
    }

    fn index(&self, indexes: Pair<usize>) -> usize {
        assert!(indexes.zip(self.dimensions).into_iter().all(|(i, d)| i < d));
        indexes.left * self.dimensions.right + indexes.right
    }

    pub fn get(&self, indexes: Pair<usize>) -> &T {
        &self.slice[self.index(indexes)]
    }

    pub fn set(&mut self, indexes: Pair<usize>, value: T) {
        let index = self.index(indexes);
        self.slice[index] = value;
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

        let matrix = (0..self.dimensions.left).map(|left| {
            (0..self.dimensions.right).map(move |right| {
                let indexes = Pair::new(left, right);
                self.get(indexes)
            })
        });

        write(matrix, f, |vector, f| {
            write(vector, f, |value, f| write!(f, "{:?}", value))
        })
    }
}
