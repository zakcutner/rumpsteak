#![cfg(feature = "serialize")]

use crate::{Branch, End, FromState, IntoSession, Receive, Role, Select, Send};
use petgraph::{dot::Dot, graph::NodeIndex, visit::EdgeRef};
use std::{
    any::{type_name, TypeId},
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Display, Formatter},
};

type Graph = petgraph::Graph<Node, Label>;

struct Type {
    id: TypeId,
    name: &'static str,
}

impl Type {
    fn new<T: 'static>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: type_name::<T>(),
        }
    }
}

enum Direction {
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

enum Node {
    Choices { role: Type, direction: Direction },
    End,
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Choices { role, direction } => write!(f, "{}{}", role.name, direction),
            Self::End => Ok(()),
        }
    }
}

struct Label(Type);

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.name)
    }
}

pub struct Serialized {
    role: Type,
    graph: Graph,
}

impl Display for Serialized {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Dot::new(&self.graph))
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
            role: Type::new::<R>(),
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
        self.serializer.previous = Some((self.node, Label(Type::new::<L>())));
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

pub fn serialize<S: FromState<'static> + Serialize>() -> Serialized {
    let mut serializer = Serializer {
        graph: petgraph::Graph::new(),
        history: HashMap::new(),
        previous: None,
    };

    S::serialize(&mut serializer);
    Serialized {
        role: Type::new::<S::Role>(),
        graph: serializer.graph,
    }
}

struct PetrifyFormatter<'a> {
    serialized: &'a Serialized,
    roles: &'a HashMap<TypeId, usize>,
    labels: &'a RefCell<HashMap<TypeId, usize>>,
}

impl<'a> PetrifyFormatter<'a> {
    fn new(
        serialized: &'a Serialized,
        roles: &'a HashMap<TypeId, usize>,
        labels: &'a RefCell<HashMap<TypeId, usize>>,
    ) -> Self {
        Self {
            serialized,
            roles,
            labels,
        }
    }

    fn label(&self, label: &Label) -> usize {
        let mut labels = self.labels.borrow_mut();
        let next_index = labels.len();
        *labels.entry(label.0.id).or_insert(next_index)
    }
}

impl Display for PetrifyFormatter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let graph = &self.serialized.graph;
        assert!(graph.node_count() > 0);

        writeln!(f, ".outputs")?;
        writeln!(f, ".state graph")?;

        for edge in graph.edge_references() {
            write!(f, "s{} ", edge.source().index())?;
            let (role, direction) = match &graph[edge.source()] {
                Node::Choices { role, direction } => (self.roles[&role.id], direction),
                _ => unreachable!(),
            };

            write!(f, "{} {} l{} ", role, direction, self.label(edge.weight()))?;
            writeln!(f, "s{}", edge.target().index())?;
        }

        writeln!(f, ".marking s0")?;
        write!(f, ".end")
    }
}

pub struct Petrify<'a> {
    serialized: &'a [Serialized],
    roles: HashMap<TypeId, usize>,
}

impl<'a> Petrify<'a> {
    pub fn new(serialized: &'a [Serialized]) -> Self {
        let roles = serialized.iter().enumerate().map(|(i, s)| (s.role.id, i));
        Self {
            serialized,
            roles: roles.collect(),
        }
    }
}

impl Display for Petrify<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let (mut serialized_iter, labels) = (self.serialized.iter(), RefCell::new(HashMap::new()));
        if let Some(serialized) = serialized_iter.next() {
            PetrifyFormatter::new(serialized, &self.roles, &labels).fmt(f)?;
            for serialized in serialized_iter {
                writeln!(f)?;
                writeln!(f)?;
                PetrifyFormatter::new(serialized, &self.roles, &labels).fmt(f)?;
            }
        }

        Ok(())
    }
}
