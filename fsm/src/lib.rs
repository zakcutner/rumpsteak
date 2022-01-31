pub mod dot;
pub mod local;
pub mod petrify;
pub mod subtype;

pub use self::{dot::Dot, local::Local, petrify::Petrify};

use petgraph::{graph::NodeIndex, visit::EdgeRef, Graph};
use std::{
    collections::HashMap,
    fmt::{self, Debug, Display, Formatter},
    hash::Hash,
};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Nil;

impl Display for Nil {
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Input,
    Output,
}

impl Action {
    fn dual(&self) -> Self {
        match self {
            Self::Input => Self::Output,
            Self::Output => Self::Input,
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input => write!(f, "?"),
            Self::Output => write!(f, "!"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Associativity {
    Left,
    Right,
}

pub trait Operator {
    fn precedence(&self) -> usize;

    fn associativity(&self) -> Associativity;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Not,
    Minus,
}

impl Operator for UnaryOp {
    fn associativity(&self) -> Associativity {
        Associativity::Right
    }

    fn precedence(&self) -> usize {
        2
    }
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Not => write!(f, "!"),
            Self::Minus => write!(f, "-"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    LAnd,
    LOr,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    And,
    Xor,
    Or,
}

impl Operator for BinaryOp {
    fn associativity(&self) -> Associativity {
        match self {
            Self::LAnd => Associativity::Left,
            Self::LOr => Associativity::Left,
            Self::Equal => Associativity::Left,
            Self::NotEqual => Associativity::Left,
            Self::Less => Associativity::Left,
            Self::Greater => Associativity::Left,
            Self::LessEqual => Associativity::Left,
            Self::GreaterEqual => Associativity::Left,
            Self::Add => Associativity::Left,
            Self::Subtract => Associativity::Left,
            Self::Multiply => Associativity::Left,
            Self::Divide => Associativity::Left,
            Self::And => Associativity::Left,
            Self::Xor => Associativity::Left,
            Self::Or => Associativity::Left,
        }
    }

    fn precedence(&self) -> usize {
        match self {
            Self::LAnd => 11,
            Self::LOr => 12,
            Self::Equal => 7,
            Self::NotEqual => 7,
            Self::Less => 6,
            Self::Greater => 6,
            Self::LessEqual => 6,
            Self::GreaterEqual => 6,
            Self::Add => 4,
            Self::Subtract => 4,
            Self::Multiply => 3,
            Self::Divide => 3,
            Self::And => 8,
            Self::Xor => 9,
            Self::Or => 10,
        }
    }
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::LAnd => write!(f, "&&"),
            Self::LOr => write!(f, "||"),
            Self::Equal => write!(f, "="),
            Self::NotEqual => write!(f, "<>"),
            Self::Less => write!(f, "<"),
            Self::Greater => write!(f, ">"),
            Self::LessEqual => write!(f, "<="),
            Self::GreaterEqual => write!(f, ">="),
            Self::Add => write!(f, "+"),
            Self::Subtract => write!(f, "-"),
            Self::Multiply => write!(f, "*"),
            Self::Divide => write!(f, "/"),
            Self::And => write!(f, "&"),
            Self::Xor => write!(f, "^"),
            Self::Or => write!(f, "|"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Expression<N> {
    Name(N),
    Boolean(bool),
    Number(usize),
    Unary(UnaryOp, Box<Self>),
    Binary(BinaryOp, Box<Self>, Box<Self>),
}

impl<N: Display> Expression<N> {
    fn fmt_bracketed(
        &self,
        f: &mut Formatter<'_>,
        associativity: Associativity,
        precedence: usize,
        op: &impl Operator,
        fmt: impl FnOnce(&mut Formatter<'_>) -> fmt::Result,
    ) -> fmt::Result {
        if op.precedence() > precedence
            || (op.precedence() == precedence && op.associativity() == associativity)
        {
            write!(f, "(")?;
            fmt(f)?;
            return write!(f, ")");
        }

        fmt(f)
    }

    fn fmt_inner(
        &self,
        f: &mut Formatter<'_>,
        associativity: Associativity,
        precedence: usize,
    ) -> fmt::Result {
        match self {
            Self::Name(name) => write!(f, "{}", name),
            Self::Boolean(boolean) => write!(f, "{}", boolean),
            Self::Number(number) => write!(f, "{}", number),
            Self::Unary(op, expression) => {
                self.fmt_bracketed(f, associativity, precedence, op, |f| {
                    write!(f, "{}", op)?;
                    expression.fmt_inner(f, Associativity::Left, op.precedence())
                })
            }
            Self::Binary(op, left, right) => {
                self.fmt_bracketed(f, associativity, precedence, op, |f| {
                    left.fmt_inner(f, Associativity::Right, op.precedence())?;
                    write!(f, " {} ", op)?;
                    right.fmt_inner(f, Associativity::Left, op.precedence())
                })
            }
        }
    }
}

impl<N: Display> Display for Expression<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_inner(f, Associativity::Left, usize::MAX)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NamedParameter<N, E> {
    name: N,
    sort: N,
    refinement: Option<E>,
}

impl<N, E> NamedParameter<N, E> {
    pub fn new(name: N, sort: N, refinement: Option<E>) -> Self {
        Self {
            name,
            sort,
            refinement,
        }
    }
}

impl<N: Display, E: Display> Display for NamedParameter<N, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.sort)?;
        if let Some(refinement) = &self.refinement {
            write!(f, "{{{}}}", refinement)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Parameters<N, E> {
    Unnamed(Vec<N>),
    Named(Vec<NamedParameter<N, E>>),
}

impl<N, E> Default for Parameters<N, E> {
    fn default() -> Self {
        Self::Unnamed(Vec::new())
    }
}

impl<N, E> Parameters<N, E> {
    pub fn is_empty(&self) -> bool {
        match self {
            Parameters::Unnamed(parameters) => parameters.is_empty(),
            Parameters::Named(parameters) => parameters.is_empty(),
        }
    }
}

impl<N: Display, E: Display> Display for Parameters<N, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn fmt<T: Display>(values: &[T], f: &mut Formatter<'_>) -> fmt::Result {
            let mut values = values.iter();
            if let Some(value) = values.next() {
                write!(f, "{}", value)?;
                for value in values {
                    write!(f, ", {}", value)?;
                }
            }

            Ok(())
        }

        match self {
            Self::Unnamed(params) => fmt(params, f),
            Self::Named(params) => fmt(params, f),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Message<N, E> {
    label: N,
    parameters: Parameters<N, E>,
    assignments: Vec<(N, E)>,
}

impl<N, E> Message<N, E> {
    pub fn new(label: N, parameters: Parameters<N, E>, assignments: Vec<(N, E)>) -> Self {
        Self {
            label,
            parameters,
            assignments,
        }
    }

    pub fn from_label(label: N) -> Self {
        Self::new(label, Default::default(), Default::default())
    }
}

impl<N: Display, E: Display> Display for Message<N, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)?;
        if !self.parameters.is_empty() {
            write!(f, "({})", self.parameters)?;
        }

        let mut assignments = self.assignments.iter();
        if let Some((name, refinement)) = assignments.next() {
            write!(f, "[{}: {}", name, refinement)?;
            for (name, refinement) in assignments {
                write!(f, ", {}: {}", name, refinement)?;
            }

            write!(f, "]")?;
        }

        Ok(())
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
pub struct Transition<R, N, E> {
    pub role: R,
    pub action: Action,
    pub message: Message<N, E>,
}

impl<R, N, E> Transition<R, N, E> {
    pub fn new(role: R, action: Action, message: Message<N, E>) -> Self {
        Self {
            role,
            action,
            message,
        }
    }

    pub fn as_ref(&self) -> TransitionRef<'_, R, N, E> {
        TransitionRef::new(&self.role, self.action, &self.message)
    }
}

impl<R: Display, N: Display, E: Display> Display for Transition<R, N, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.role, self.action, self.message)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TransitionRef<'a, R, N, E> {
    pub role: &'a R,
    pub action: Action,
    pub message: &'a Message<N, E>,
}

impl<'a, R, N, E> TransitionRef<'a, R, N, E> {
    pub fn new(role: &'a R, action: Action, message: &'a Message<N, E>) -> Self {
        Self {
            role,
            action,
            message,
        }
    }
}

impl<R, N, E> Clone for TransitionRef<'_, R, N, E> {
    fn clone(&self) -> Self {
        Self::new(self.role, self.action, self.message)
    }
}

impl<R: Clone, N: Clone, E: Clone> TransitionRef<'_, R, N, E> {
    pub fn to_owned(&self) -> Transition<R, N, E> {
        Transition::new(self.role.clone(), self.action, self.message.clone())
    }
}

impl<R: Display, N: Display, E: Display> Display for TransitionRef<'_, R, N, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.role, self.action, self.message)
    }
}

#[derive(Debug, Error)]
pub enum AddTransitionError {
    #[error("cannot perform self-communication")]
    SelfCommunication,
    #[error("cannot communicate with different roles from the same state")]
    MultipleRoles,
    #[error("cannot both send and receive from the same state")]
    MultipleActions,
}

#[derive(Clone, Debug)]
pub struct Fsm<R, N, E> {
    role: R,
    graph: Graph<State<R>, Message<N, E>>,
}

impl<R, N, E> Fsm<R, N, E> {
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
    ) -> impl Iterator<Item = (StateIndex, StateIndex, TransitionRef<R, N, E>)> {
        self.graph.edge_references().map(move |edge| {
            let (source, target) = (StateIndex(edge.source()), StateIndex(edge.target()));
            match &self.graph[edge.source()] {
                State::Choices(choices) => {
                    let transition =
                        TransitionRef::new(&choices.role, choices.action, edge.weight());
                    (source, target, transition)
                }
                _ => unreachable!(),
            }
        })
    }

    pub fn transitions_from(
        &self,
        StateIndex(index): StateIndex,
    ) -> impl Iterator<Item = (StateIndex, TransitionRef<R, N, E>)> {
        self.graph
            .edges(index)
            .map(move |edge| match &self.graph[index] {
                State::Choices(choices) => {
                    let transition =
                        TransitionRef::new(&choices.role, choices.action, edge.weight());
                    (StateIndex(edge.target()), transition)
                }
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
        transition: Transition<R, N, E>,
    ) -> Result<(), AddTransitionError>
    where
        R: Eq,
    {
        if transition.role == self.role {
            return Err(AddTransitionError::SelfCommunication);
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
                    return Err(AddTransitionError::MultipleRoles);
                }

                if choices.action != expected.action {
                    return Err(AddTransitionError::MultipleActions);
                }
            }
        }

        self.graph.add_edge(from.0, to.0, transition.message);
        Ok(())
    }

    pub fn to_binary(&self) -> Fsm<Nil, N, E>
    where
        R: Debug + Eq,
        N: Clone,
        E: Clone,
    {
        let mut role = None;
        let graph = self.graph.map(
            |_, state| match state {
                State::Choices(choice) => {
                    match role {
                        Some(role) => assert_eq!(role, &choice.role),
                        None => role = Some(&choice.role),
                    }

                    State::Choices(Choices {
                        role: Nil,
                        action: choice.action,
                    })
                }
                State::End => State::End,
            },
            |_, edge| edge.clone(),
        );

        Fsm { role: Nil, graph }
    }

    pub fn dual(&self, role: R) -> Self
    where
        R: Clone + Debug + Eq,
        N: Clone,
        E: Clone,
    {
        let graph = self.graph.map(
            |_, state| match state {
                State::Choices(choice) => {
                    assert_eq!(role, choice.role);
                    State::Choices(Choices {
                        role: self.role.clone(),
                        action: choice.action.dual(),
                    })
                }
                State::End => State::End,
            },
            |_, edge| edge.clone(),
        );

        Fsm { role, graph }
    }
}

pub struct Normalizer<'a, R, N> {
    roles: HashMap<&'a R, usize>,
    labels: HashMap<&'a N, usize>,
}

impl<R, N> Default for Normalizer<'_, R, N> {
    fn default() -> Self {
        Self {
            roles: Default::default(),
            labels: Default::default(),
        }
    }
}

impl<'a, R: Eq + Hash, N: Eq + Hash> Normalizer<'a, R, N> {
    fn role(roles: &mut HashMap<&'a R, usize>, role: &'a R) -> usize {
        let next_index = roles.len();
        *roles.entry(role).or_insert(next_index)
    }

    fn label(labels: &mut HashMap<&'a N, usize>, label: &'a N) -> usize {
        let next_index = labels.len();
        *labels.entry(label).or_insert(next_index)
    }

    pub fn normalize<E: Clone>(&mut self, input: &'a Fsm<R, N, E>) -> Fsm<usize, usize, E> {
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
                |_, message| Message::from_label(Self::label(labels, &message.label)),
            ),
        }
    }
}
