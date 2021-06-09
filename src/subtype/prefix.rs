use crate::fsm::Transition;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy)]
pub struct Index(usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    size: usize,
    start: usize,
    removed: usize,
}

#[derive(Debug)]
pub struct Prefix<'a, R, L> {
    transitions: Vec<(bool, Transition<&'a R, &'a L>)>,
    start: usize,
    removed: Vec<usize>,
}

impl<R, L> Default for Prefix<'_, R, L> {
    fn default() -> Self {
        Self {
            transitions: Default::default(),
            start: Default::default(),
            removed: Default::default(),
        }
    }
}

impl<'a, R, L> Prefix<'a, R, L> {
    pub fn is_empty(&self) -> bool {
        self.start >= self.transitions.len()
    }

    pub fn first(&self) -> Option<&Transition<&'a R, &'a L>> {
        if let Some((removed, transition)) = self.transitions.get(self.start) {
            assert!(!removed);
            return Some(transition);
        }

        None
    }

    pub fn push(&mut self, transition: Transition<&'a R, &'a L>) {
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
        L: Eq,
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

    pub fn iter_full(&self) -> impl Iterator<Item = (Index, &Transition<&'a R, &'a L>)> {
        let prefixes = self.transitions.iter().enumerate().skip(self.start);
        prefixes.filter_map(|(i, (removed, transition))| (!removed).then(|| (Index(i), transition)))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Transition<&'a R, &'a L>> {
        self.iter_full().map(|(_, transition)| transition)
    }
}

impl<R: Display, L: Display> Display for Prefix<'_, R, L> {
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
