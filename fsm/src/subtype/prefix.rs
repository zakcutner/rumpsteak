use crate::TransitionRef;
use std::{
    convert::Infallible,
    fmt::{self, Display, Formatter},
};

#[derive(Clone, Copy)]
pub struct Index(usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    size: usize,
    start: usize,
    removed: usize,
}

#[derive(Debug)]
pub struct Prefix<'a, R, N> {
    transitions: Vec<(bool, TransitionRef<'a, R, N, Infallible>)>,
    start: usize,
    removed: Vec<usize>,
}

impl<R, N> Default for Prefix<'_, R, N> {
    fn default() -> Self {
        Self {
            transitions: Default::default(),
            start: Default::default(),
            removed: Default::default(),
        }
    }
}

impl<'a, R, N> Prefix<'a, R, N> {
    pub fn is_empty(&self) -> bool {
        self.start >= self.transitions.len()
    }

    pub(super) fn first(&self) -> Option<&TransitionRef<'a, R, N, Infallible>> {
        if let Some((removed, transition)) = self.transitions.get(self.start) {
            assert!(!removed);
            return Some(transition);
        }

        None
    }

    pub(super) fn push(&mut self, transition: TransitionRef<'a, R, N, Infallible>) {
        self.transitions.push((false, transition));
    }

    pub fn remove_first(&mut self) {
        assert!(matches!(self.transitions.get(self.start), Some((false, _))));
        self.start += 1;
        while let Some((true, _)) = self.transitions.get(self.start) {
            self.start += 1;
        }
    }

    pub fn remove(&mut self, Index(i): Index) {
        if i == self.start {
            self.remove_first();
            return;
        }

        let (removed, _) = &mut self.transitions[i];
        assert!(!*removed);
        *removed = true;
        self.removed.push(i);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            size: self.transitions.len(),
            start: self.start,
            removed: self.removed.len(),
        }
    }

    fn valid_snapshot(&self, snapshot: &Snapshot) -> bool {
        snapshot.removed <= self.removed.len()
            && snapshot.size <= self.transitions.len()
            && snapshot.start <= self.start
    }

    pub fn is_modified(&self, snapshot: &Snapshot) -> bool
    where
        R: Eq,
        N: Eq,
    {
        assert!(self.valid_snapshot(snapshot));
        self.transitions[self.start..] != self.transitions[..snapshot.size][snapshot.start..]
    }

    pub fn revert(&mut self, snapshot: &Snapshot) {
        assert!(self.valid_snapshot(snapshot));
        for &i in self.removed.get(snapshot.removed..).unwrap_or_default() {
            let (removed, _) = &mut self.transitions[i];
            assert!(*removed);
            *removed = false;
        }

        self.removed.truncate(snapshot.removed);
        self.transitions.truncate(snapshot.size);
        self.start = snapshot.start;
    }

    pub(super) fn iter_full(
        &self,
    ) -> impl Iterator<Item = (Index, &TransitionRef<'a, R, N, Infallible>)> {
        let prefixes = self.transitions.iter().enumerate().skip(self.start);
        prefixes.filter_map(|(i, (removed, transition))| (!removed).then(|| (Index(i), transition)))
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &TransitionRef<'a, R, N, Infallible>> {
        self.iter_full().map(|(_, transition)| transition)
    }
}

impl<R: Display, N: Display> Display for Prefix<'_, R, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut transitions = self.iter();
        if let Some(transition) = transitions.next() {
            write!(f, "{}", transition)?;
            for transition in transitions {
                write!(f, " . {}", transition)?;
            }

            return Ok(());
        }

        write!(f, "empty")
    }
}
