#![cfg(feature = "subtyping")]

mod matrix;
mod pair;

use self::{matrix::Matrix, pair::Pair};
use crate::fsm::{Action, Fsm, StateIndex, Transition};
use std::{
    fmt::{self, Display, Formatter},
    iter::Peekable,
};

#[derive(Clone, Copy)]
struct TransitionIndex(usize);

#[derive(Debug, PartialEq, Eq)]
struct PrefixSnapshot {
    size: usize,
    start: usize,
    removed: usize,
}

#[derive(Debug)]
struct Prefix<'a, R, L> {
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
    fn is_empty(&self) -> bool {
        self.start >= self.transitions.len()
    }

    fn first(&self) -> Option<&Transition<&'a R, &'a L>> {
        if let Some((removed, transition)) = self.transitions.get(self.start) {
            assert!(!removed);
            return Some(transition);
        }

        None
    }

    fn push(&mut self, transition: Transition<&'a R, &'a L>) {
        self.transitions.push((false, transition));
    }

    fn remove_first(&mut self) {
        assert!(matches!(self.transitions.get(self.start), Some((false, _))));
        self.start += 1;
        while let Some((true, _)) = self.transitions.get(self.start) {
            self.start += 1;
        }
    }

    fn remove(&mut self, TransitionIndex(i): TransitionIndex) {
        if i == self.start {
            self.remove_first();
            return;
        }

        let (removed, _) = &mut self.transitions[i];
        assert!(!*removed);
        *removed = true;
        self.removed.push(i);
    }

    fn snapshot(&self) -> PrefixSnapshot {
        PrefixSnapshot {
            size: self.transitions.len(),
            start: self.start,
            removed: self.removed.len(),
        }
    }

    fn restore(&mut self, snapshot: &PrefixSnapshot) {
        for &i in self.removed.get(snapshot.removed..).unwrap_or_default() {
            let (removed, _) = &mut self.transitions[i];
            assert!(*removed);
            *removed = false;
        }

        assert!(snapshot.removed <= self.removed.len());
        self.removed.truncate(snapshot.removed);

        assert!(snapshot.size <= self.transitions.len());
        self.transitions.truncate(snapshot.size);

        assert!(snapshot.start <= self.start);
        self.start = snapshot.start;
    }

    fn iter(&self) -> impl Iterator<Item = (TransitionIndex, &Transition<&'a R, &'a L>)> {
        let prefixes = self.transitions.iter().enumerate().skip(self.start);
        prefixes.filter_map(|(i, (removed, transition))| {
            (!removed).then(|| (TransitionIndex(i), transition))
        })
    }
}

impl<R: Display, L: Display> Display for Prefix<'_, R, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut transitions = self.iter().map(|(_, transition)| transition);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Quantifier {
    All,
    Any,
}

struct SubtypeVisitor<'a, R, L> {
    fsms: Pair<&'a Fsm<R, L>>,
    history: Matrix<bool>,
    visits: Pair<Box<[usize]>>,
    prefixes: Pair<Prefix<'a, R, L>>,
}

impl<'a, R: Eq, L: Eq> SubtypeVisitor<'a, R, L> {
    #[inline]
    fn unroll<I: Iterator<Item = (StateIndex, Transition<&'a R, &'a L>)>, const SWAP: bool>(
        &mut self,
        mut transitions: Pair<I>,
        mut quantifiers: Pair<Quantifier>,
    ) -> bool {
        let mut prefixes = self.prefixes.as_ref();
        if SWAP {
            prefixes.swap();
            transitions.swap();
            quantifiers.swap();
        }

        let right_transitions = transitions.right.collect::<Vec<_>>();
        let snapshots = prefixes.map(Prefix::snapshot);

        for (left_state, left_transition) in transitions.left {
            let mut prefixes = self.prefixes.as_mut();
            if SWAP {
                prefixes.swap();
            }

            prefixes.left.restore(&snapshots.left);
            prefixes.left.push(left_transition);

            let mut output = quantifiers.right == Quantifier::All;
            for (right_state, right_transition) in &right_transitions {
                let mut prefixes = self.prefixes.as_mut();
                if SWAP {
                    prefixes.swap();
                }

                prefixes.right.restore(&snapshots.right);
                prefixes.right.push(right_transition.clone());

                let mut states = Pair::new(left_state, *right_state);
                if SWAP {
                    states.swap();
                }

                output = self.visit(states);

                if output == (quantifiers.right == Quantifier::Any) {
                    break;
                }
            }

            if output == (quantifiers.left == Quantifier::Any) {
                return output;
            }
        }

        quantifiers.left == Quantifier::All
    }

    fn visit(&mut self, states: Pair<StateIndex>) -> bool {
        let indexes = states.map(StateIndex::index);

        let visits = self.visits.as_ref().zip(indexes);
        if visits.into_iter().any(|(v, i)| v[i] == 0) {
            return false;
        }

        if !reduce(&mut self.prefixes) {
            return false;
        }

        let empty_prefixes = self.prefixes.iter().all(Prefix::is_empty);
        let mut transitions = self.fsms.zip(states).map(|(fsm, state)| {
            let transitions = fsm.transitions_from(state);
            transitions.peekable()
        });

        match transitions.as_mut().map(Peekable::peek).into() {
            (None, None) if empty_prefixes => true,
            (Some((_, left)), Some((_, right))) => {
                let in_history = self.history[indexes];
                if in_history && empty_prefixes {
                    return true;
                }

                self.history[indexes] = true;
                for (visits, i) in self.visits.as_mut().zip(indexes).into_iter() {
                    visits[i] -= 1;
                }

                let output = match (left.action, right.action) {
                    (Action::Output, Action::Output) => {
                        let quantifiers = Pair::new(Quantifier::All, Quantifier::Any);
                        self.unroll::<_, false>(transitions, quantifiers)
                    }
                    (Action::Output, Action::Input) => {
                        let quantifiers = Pair::new(Quantifier::All, Quantifier::All);
                        self.unroll::<_, false>(transitions, quantifiers)
                    }
                    (Action::Input, Action::Output) => {
                        let quantifiers = Pair::new(Quantifier::Any, Quantifier::Any);
                        self.unroll::<_, false>(transitions, quantifiers)
                    }
                    (Action::Input, Action::Input) => {
                        let quantifiers = Pair::new(Quantifier::Any, Quantifier::All);
                        self.unroll::<_, true>(transitions, quantifiers)
                    }
                };

                self.history[indexes] = in_history;
                for (visits, i) in self.visits.as_mut().zip(indexes).into_iter() {
                    visits[i] += 1;
                }

                output
            }
            _ => false,
        }
    }
}

fn reduce<R: Eq, L: Eq>(prefixes: &mut Pair<Prefix<R, L>>) -> bool {
    fn reorder<R: Eq, L: Eq>(
        left: &Transition<&R, &L>,
        rights: &Prefix<R, L>,
        reject: impl Fn(&Transition<&R, &L>, &Transition<&R, &L>) -> bool,
    ) -> Option<Option<TransitionIndex>> {
        let mut rights = rights.iter();

        let (_, right) = rights.next().unwrap();
        if reject(left, right) {
            return None;
        }

        for (i, right) in rights {
            if left == right {
                return Some(Some(i));
            }

            if reject(left, right) {
                return None;
            }
        }

        Some(None)
    }

    while let (Some(left), Some(right)) = prefixes.as_ref().map(Prefix::first).into() {
        // Fast path to avoid added control flow.
        if left == right {
            for prefix in prefixes.iter_mut() {
                prefix.remove_first();
            }

            continue;
        }

        // TODO: cache the results of these checks to only search new actions.
        let i = match left.action {
            Action::Input => reorder(left, &prefixes.right, |left, right| {
                right.role == left.role || right.action == Action::Output
            }),
            Action::Output => reorder(left, &prefixes.right, |left, right| {
                right.role == left.role && right.action == Action::Output
            }),
        };

        match i {
            Some(Some(i)) => {
                prefixes.left.remove_first();
                prefixes.right.remove(i);
                continue;
            }
            Some(None) => break,
            None => return false,
        }
    }

    true
}

pub fn is_subtype<R: Eq, L: Eq>(left: &Fsm<R, L>, right: &Fsm<R, L>, visits: usize) -> bool {
    if left.role() != right.role() {
        panic!("FSMs are for different roles");
    }

    let sizes = Pair::new(left.size().0, right.size().0);
    let mut visitor = SubtypeVisitor {
        fsms: Pair::new(left, right),
        history: Matrix::new(sizes),
        visits: sizes.map(|size| vec![visits; size].into_boxed_slice()),
        prefixes: Default::default(),
    };

    visitor.visit(Default::default())
}
