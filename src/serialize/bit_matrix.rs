use super::pair::Pair;
use bitvec::{bitbox, boxed::BitBox};

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
