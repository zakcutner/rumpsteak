#![cfg(feature = "fsm")]

pub mod dot;
pub mod local;
pub mod petrify;

pub use self::dot::Dot;
pub use self::petrify::Petrify;

use petgraph::{graph::NodeIndex, visit::EdgeRef, Graph};
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    hash::Hash,
};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Input,
    Output,
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input => write!(f, "?"),
            Self::Output => write!(f, "!"),
        }
    }
}

#[derive(Clone, Debug)]
struct Choices<R> {
    role: R,
    action: Action,
}

#[derive(Clone, Debug)]
enum State<R> {
    Choices(Choices<R>),
    End,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct StateIndex(NodeIndex);

impl StateIndex {
    pub(crate) fn index(self) -> usize {
        self.0.index()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transition<R, L> {
    pub role: R,
    pub action: Action,
    pub label: L,
}

impl<R, L> Transition<R, L> {
    pub fn new(role: R, action: Action, label: L) -> Self {
        Self {
            role,
            action,
            label,
        }
    }
}

impl<R: Clone, L: Clone> Transition<&R, &L> {
    fn to_owned(&self) -> Transition<R, L> {
        Transition::new(self.role.clone(), self.action, self.label.clone())
    }
}

impl<R: Display, L: Display> Display for Transition<R, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.role, self.action, self.label)
    }
}

#[derive(Debug, Error)]
pub enum TransitionError {
    #[error("cannot perform self-communication")]
    SelfCommunication,
    #[error("cannot communicate with different roles from the same state")]
    MultipleRoles,
    #[error("cannot both send and receive from the same state")]
    MultipleActions,
}

#[derive(Clone, Debug)]
pub struct Fsm<R, L> {
    role: R,
    graph: Graph<State<R>, L>,
}

impl<R, L> Fsm<R, L> {
    pub fn new(role: R) -> Self {
        let graph = Graph::new();
        Self { role, graph }
    }

    pub fn role(&self) -> &R {
        &self.role
    }

    pub fn size(&self) -> (usize, usize) {
        (self.graph.node_count(), self.graph.edge_count())
    }

    pub fn states(&self) -> impl Iterator<Item = StateIndex> {
        self.graph.node_indices().map(StateIndex)
    }

    pub fn transitions(
        &self,
    ) -> impl Iterator<Item = (StateIndex, StateIndex, Transition<&R, &L>)> {
        self.graph
            .edge_references()
            .map(move |edge| match &self.graph[edge.source()] {
                State::Choices(choices) => (
                    StateIndex(edge.source()),
                    StateIndex(edge.target()),
                    Transition::new(&choices.role, choices.action, edge.weight()),
                ),
                _ => unreachable!(),
            })
    }

    pub fn transitions_from(
        &self,
        StateIndex(index): StateIndex,
    ) -> impl Iterator<Item = (StateIndex, Transition<&R, &L>)> {
        self.graph
            .edges(index)
            .map(move |edge| match &self.graph[index] {
                State::Choices(choices) => (
                    StateIndex(edge.target()),
                    Transition::new(&choices.role, choices.action, edge.weight()),
                ),
                _ => unreachable!(),
            })
    }

    pub fn add_state(&mut self) -> StateIndex {
        StateIndex(self.graph.add_node(State::End))
    }

    pub fn add_transition(
        &mut self,
        from: StateIndex,
        to: StateIndex,
        transition: Transition<R, L>,
    ) -> Result<(), TransitionError>
    where
        R: Eq,
    {
        if transition.role == self.role {
            return Err(TransitionError::SelfCommunication);
        }

        let choices = Choices {
            role: transition.role,
            action: transition.action,
        };

        let state = &mut self.graph[from.0];
        match state {
            State::End => *state = State::Choices(choices),
            State::Choices(expected) => {
                if choices.role != expected.role {
                    return Err(TransitionError::MultipleRoles);
                }

                if choices.action != expected.action {
                    return Err(TransitionError::MultipleActions);
                }
            }
        }

        self.graph.add_edge(from.0, to.0, transition.label);
        Ok(())
    }
}

pub struct Normalizer<'a, R, L> {
    roles: HashMap<&'a R, usize>,
    labels: HashMap<&'a L, usize>,
}

impl<R, L> Default for Normalizer<'_, R, L> {
    fn default() -> Self {
        Self {
            roles: Default::default(),
            labels: Default::default(),
        }
    }
}

impl<'a, R: Eq + Hash, L: Eq + Hash> Normalizer<'a, R, L> {
    fn role(roles: &mut HashMap<&'a R, usize>, role: &'a R) -> usize {
        let next_index = roles.len();
        *roles.entry(role).or_insert(next_index)
    }

    fn label(labels: &mut HashMap<&'a L, usize>, label: &'a L) -> usize {
        let next_index = labels.len();
        *labels.entry(label).or_insert(next_index)
    }

    pub fn normalize(&mut self, input: &'a Fsm<R, L>) -> Fsm<usize, usize> {
        let (roles, labels) = (&mut self.roles, &mut self.labels);
        Fsm {
            role: Self::role(roles, &input.role),
            graph: input.graph.map(
                |_, state| match state {
                    State::End => State::End,
                    State::Choices(choices) => State::Choices(Choices {
                        role: Self::role(roles, &choices.role),
                        action: choices.action,
                    }),
                },
                |_, label| Self::label(labels, &label),
            ),
        }
    }
}
