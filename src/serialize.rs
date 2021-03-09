#![cfg(feature = "serialize")]

use crate::{Branch, End, FromState, IntoSession, Receive, Role, Select, Send};
use petgraph::graph::NodeIndex;
use std::{
    any::{type_name, TypeId},
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Display, Formatter},
};

pub type Graph = petgraph::Graph<Node, Label>;

pub enum Direction {
    Send,
    Receive,
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Send => write!(f, "!"),
            Self::Receive => write!(f, "?"),
        }
    }
}

pub enum Node {
    Choices {
        role: &'static str,
        direction: Direction,
    },
    End,
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Choices { role, direction } => write!(f, "{}{}", role, direction),
            Self::End => Ok(()),
        }
    }
}

pub struct Label(&'static str);

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Serializer {
    graph: Graph,
    history: HashMap<TypeId, NodeIndex>,
    previous: Option<(NodeIndex, Label)>,
}

impl Serializer {
    fn add_node_index(&mut self, node: NodeIndex) {
        if let Some((previous, edge)) = self.previous.take() {
            self.graph.add_edge(previous, node, edge);
        }
    }

    fn add_node<S: 'static>(&mut self, node: Node) -> Option<NodeIndex> {
        match self.history.entry(TypeId::of::<S>()) {
            Entry::Occupied(entry) => {
                let node = *entry.get();
                self.add_node_index(node);
                None
            }
            Entry::Vacant(entry) => {
                let node = self.graph.add_node(node);
                entry.insert(node);
                self.add_node_index(node);
                Some(node)
            }
        }
    }

    fn serialize_end<S: 'static>(&mut self) {
        self.add_node::<S>(Node::End);
    }

    fn serialize_choices<S: 'static, R: 'static>(
        &mut self,
        direction: Direction,
    ) -> Option<ChoicesSerializer> {
        self.add_node::<S>(Node::Choices {
            role: type_name::<R>(),
            direction,
        })
        .map(move |node| ChoicesSerializer {
            serializer: self,
            node,
        })
    }
}

pub struct ChoicesSerializer<'a> {
    serializer: &'a mut Serializer,
    node: NodeIndex,
}

impl ChoicesSerializer<'_> {
    pub fn serialize_choice<L: 'static, S: Serialize>(&mut self) {
        self.serializer.previous = Some((self.node, Label(type_name::<L>())));
        S::serialize(&mut self.serializer);
    }
}

pub trait Serialize: 'static {
    fn serialize(s: &mut Serializer);
}

pub trait SerializeChoices: 'static {
    fn serialize_choices(s: ChoicesSerializer<'_>);
}

impl<S: IntoSession<'static> + 'static> Serialize for S
where
    S::Session: Serialize,
{
    fn serialize(s: &mut Serializer) {
        S::Session::serialize(s);
    }
}

impl<R: Role + 'static> Serialize for End<'static, R> {
    fn serialize(s: &mut Serializer) {
        s.serialize_end::<Self>();
    }
}

impl<Q: Role + 'static, R: 'static, L: 'static, S> Serialize for Send<'static, Q, R, L, S>
where
    S: FromState<'static, Role = Q> + Serialize + 'static,
{
    fn serialize(s: &mut Serializer) {
        if let Some(mut s) = s.serialize_choices::<Self, R>(Direction::Send) {
            s.serialize_choice::<L, S>();
        }
    }
}

impl<Q: Role + 'static, R: 'static, L: 'static, S> Serialize for Receive<'static, Q, R, L, S>
where
    S: FromState<'static, Role = Q> + Serialize + 'static,
{
    fn serialize(s: &mut Serializer) {
        if let Some(mut s) = s.serialize_choices::<Self, R>(Direction::Receive) {
            s.serialize_choice::<L, S>();
        }
    }
}

impl<Q: Role + 'static, R: 'static, C: SerializeChoices + 'static> Serialize
    for Select<'static, Q, R, C>
{
    fn serialize(s: &mut Serializer) {
        if let Some(s) = s.serialize_choices::<Self, R>(Direction::Send) {
            C::serialize_choices(s);
        }
    }
}

impl<Q: Role + 'static, R: 'static, C: SerializeChoices + 'static> Serialize
    for Branch<'static, Q, R, C>
{
    fn serialize(s: &mut Serializer) {
        if let Some(s) = s.serialize_choices::<Self, R>(Direction::Receive) {
            C::serialize_choices(s);
        }
    }
}

pub fn serialize<S: Serialize>() -> Graph {
    let mut serializer = Serializer {
        graph: Graph::new(),
        history: HashMap::new(),
        previous: None,
    };

    S::serialize(&mut serializer);
    serializer.graph
}
