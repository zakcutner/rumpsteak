#![cfg(feature = "subtyping")]

mod matrix;
mod pair;
mod prefix;

use self::{
    matrix::Matrix,
    pair::Pair,
    prefix::{Index, Prefix, Snapshot},
};
use crate::fsm::{Action, Fsm, StateIndex, Transition};
use std::{iter::Peekable, mem};

#[derive(Clone)]
struct Previous {
    visits: usize,
    snapshots: Option<Pair<Snapshot>>,
}

impl Previous {
    fn new(visits: usize, snapshots: Option<Pair<Snapshot>>) -> Self {
        Self { visits, snapshots }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Quantifier {
    All,
    Any,
}

struct SubtypeVisitor<'a, R, L> {
    fsms: Pair<&'a Fsm<R, L>>,
    history: Matrix<Previous>,
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
        let left_snapshot = prefixes.left.snapshot();
        let right_snapshot = prefixes.right.snapshot();

        for (left_state, left_transition) in transitions.left {
            let mut prefixes = self.prefixes.as_mut();
            if SWAP {
                prefixes.swap();
            }

            prefixes.left.revert(&left_snapshot);
            prefixes.left.push(left_transition);
            let left_snapshot = prefixes.left.snapshot();

            let mut output = quantifiers.right == Quantifier::All;
            for (right_state, right_transition) in &right_transitions {
                let mut prefixes = self.prefixes.as_mut();
                if SWAP {
                    prefixes.swap();
                }

                prefixes.left.revert(&left_snapshot);
                prefixes.right.revert(&right_snapshot);
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
        let index = states.map(StateIndex::index);
        if self.history[index].visits == 0 {
            return false;
        }

        if !reduce(&mut self.prefixes) {
            return false;
        }

        if let Some(snapshots) = &self.history[index].snapshots {
            let mut prefixes = self.prefixes.as_ref().zip(snapshots.as_ref()).into_iter();
            if prefixes.all(|(prefix, snapshot)| !prefix.is_modified(snapshot)) {
                return true;
            }
        }

        let mut transitions = self.fsms.zip(states).map(|(fsm, state)| {
            let transitions = fsm.transitions_from(state);
            transitions.peekable()
        });

        let empty_prefixes = self.prefixes.iter().all(Prefix::is_empty);
        match transitions.as_mut().map(Peekable::peek).into() {
            (None, None) if empty_prefixes => true,
            (Some((_, left)), Some((_, right))) => {
                let snapshots = self.prefixes.as_ref().map(Prefix::snapshot);
                let previous = Previous::new(self.history[index].visits - 1, Some(snapshots));
                let previous = mem::replace(&mut self.history[index], previous);

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

                self.history[index] = previous;
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
    ) -> Option<Option<Index>> {
        let mut rights = rights.iter_full();

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
        history: Matrix::new(sizes, Previous::new(visits, None)),
        prefixes: Default::default(),
    };

    visitor.visit(Default::default())
}
