use super::pair::Pair;
use bitvec::{bitbox, boxed::BitBox};
use std::fmt::{self, Debug, Formatter};

pub struct BitMatrix {
    dimensions: Pair<usize>,
    slice: BitBox,
}

impl BitMatrix {
    pub fn new(dimensions: Pair<usize>) -> Self {
        Self {
            dimensions,
            slice: bitbox![0; dimensions.left * dimensions.right],
        }
    }

    fn index(&self, indexes: Pair<usize>) -> usize {
        assert!(indexes.zip(self.dimensions).into_iter().all(|(i, d)| i < d));
        indexes.left * self.dimensions.right + indexes.right
    }

    pub fn get(&self, indexes: Pair<usize>) -> bool {
        self.slice[self.index(indexes)]
    }

    pub fn set(&mut self, indexes: Pair<usize>, value: bool) {
        let index = self.index(indexes);
        self.slice.set(index, value);
    }
}

impl Debug for BitMatrix {
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
            write(vector, f, |b, f| match b {
                true => write!(f, "1"),
                false => write!(f, "0"),
            })
        })
    }
}
