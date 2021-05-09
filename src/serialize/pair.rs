use std::{
    fmt::{self, Display, Formatter},
    mem,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pair<T> {
    pub left: T,
    pub right: T,
}

impl<T> Pair<T> {
    pub fn new(left: T, right: T) -> Self {
        Self { left, right }
    }

    pub fn as_ref(&self) -> Pair<&T> {
        Pair::new(&self.left, &self.right)
    }

    pub fn as_mut(&mut self) -> Pair<&mut T> {
        Pair::new(&mut self.left, &mut self.right)
    }

    pub fn swap(&mut self) {
        mem::swap(&mut self.left, &mut self.right)
    }

    pub fn zip<U>(self, other: Pair<U>) -> Pair<(T, U)> {
        Pair::new((self.left, other.left), (self.right, other.right))
    }

    pub fn map<U>(self, f: impl Fn(T) -> U) -> Pair<U> {
        Pair::new(f(self.left), f(self.right))
    }

    pub fn into_iter(self) -> impl Iterator<Item = T> {
        self.map(Some)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_ref().into_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.as_mut().into_iter()
    }
}

impl<T: Display> Display for Pair<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{}, {}>", self.left, self.right)
    }
}

impl<T> Iterator for Pair<Option<T>> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.left.take().or_else(|| self.right.take())
    }
}

impl<T> From<Pair<T>> for (T, T) {
    fn from(pair: Pair<T>) -> Self {
        (pair.left, pair.right)
    }
}

impl<T> From<Pair<T>> for [T; 2] {
    fn from(pair: Pair<T>) -> Self {
        [pair.left, pair.right]
    }
}
