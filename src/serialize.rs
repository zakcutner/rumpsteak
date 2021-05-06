#![cfg(feature = "serialize")]

use crate::{Branch, End, FromState, Receive, Role, Select, Send};
use bitvec::{bitbox, boxed::BitBox};
use fmt::Debug;
use petgraph::{dot::Dot, graph::NodeIndex, visit::EdgeRef};
use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    cell::RefCell,
    collections::{hash_map::Entry, HashMap, VecDeque},
    convert::identity,
    fmt::{self, Display, Formatter},
};

type Graph = petgraph::Graph<Node, Label>;

#[derive(Clone, Copy, Eq)]
struct Type {
    id: TypeId,
    name: &'static str,
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Type {
    fn new<T: 'static>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: type_name::<T>(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Label(Type);

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
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

impl<R: Role + 'static> Serialize for End<'static, R> {
    fn serialize(s: &mut Serializer) {
        s.serialize_end::<Self>();
    }
}

impl<Q: Role + 'static, R: 'static, L: 'static, S> Serialize for Send<'static, Q, R, L, S>
where
    S: FromState<'static, Role = Q> + Serialize,
{
    fn serialize(s: &mut Serializer) {
        if let Some(mut s) = s.serialize_choices::<Self, R>(Direction::Send) {
            s.serialize_choice::<L, S>();
        }
    }
}

impl<Q: Role + 'static, R: 'static, L: 'static, S> Serialize for Receive<'static, Q, R, L, S>
where
    S: FromState<'static, Role = Q> + Serialize,
{
    fn serialize(s: &mut Serializer) {
        if let Some(mut s) = s.serialize_choices::<Self, R>(Direction::Receive) {
            s.serialize_choice::<L, S>();
        }
    }
}

impl<Q: Role + 'static, R: 'static, C: SerializeChoices> Serialize for Select<'static, Q, R, C> {
    fn serialize(s: &mut Serializer) {
        if let Some(s) = s.serialize_choices::<Self, R>(Direction::Send) {
            C::serialize_choices(s);
        }
    }
}

impl<Q: Role + 'static, R: 'static, C: SerializeChoices> Serialize for Branch<'static, Q, R, C> {
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

    fn label(&self, label: Label) -> usize {
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

            write!(f, "{} {} l{} ", role, direction, self.label(*edge.weight()))?;
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Pair<T> {
    left: T,
    right: T,
}

impl<T> Pair<T> {
    fn new(left: T, right: T) -> Self {
        Self { left, right }
    }

    fn as_ref(&self) -> Pair<&T> {
        Pair::new(&self.left, &self.right)
    }

    fn as_mut(&mut self) -> Pair<&mut T> {
        Pair::new(&mut self.left, &mut self.right)
    }

    fn swap(self) -> Self {
        Self::new(self.right, self.left)
    }

    fn zip<U>(self, other: Pair<U>) -> Pair<(T, U)> {
        Pair::new((self.left, other.left), (self.right, other.right))
    }

    fn map<U>(self, f: impl Fn(T) -> U) -> Pair<U> {
        Pair::new(f(self.left), f(self.right))
    }

    fn into_iter(self) -> impl Iterator<Item = T> {
        self.map(Option::Some)
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_ref().into_iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.as_mut().into_iter()
    }

    fn into_tuple(self) -> (T, T) {
        (self.left, self.right)
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

struct BitMatrix {
    dimensions: Pair<usize>,
    slice: BitBox,
}

impl BitMatrix {
    fn new(dimensions: Pair<usize>) -> Self {
        Self {
            dimensions,
            slice: bitbox![0; dimensions.left * dimensions.right],
        }
    }

    fn index(&self, indexes: Pair<usize>) -> usize {
        assert!(indexes.zip(self.dimensions).into_iter().all(|(i, d)| i < d));
        indexes.left * self.dimensions.right + indexes.right
    }

    fn get(&self, indexes: Pair<usize>) -> bool {
        self.slice[self.index(indexes)]
    }

    fn set(&mut self, indexes: Pair<usize>, value: bool) {
        let index = self.index(indexes);
        self.slice.set(index, value);
    }
}

// TODO: consider making a copy type.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Prefix {
    role: Type,
    direction: Direction,
    label: Label,
}

impl Display for Prefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.role, self.direction, self.label)
    }
}

#[derive(Clone, Debug, Default)]
struct Prefixes {
    queue: VecDeque<Prefix>,
}

impl Display for Prefixes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut prefixes = self.queue.iter();
        if let Some(prefix) = prefixes.next() {
            write!(f, "{}", prefix)?;
            for prefix in prefixes {
                write!(f, " . {}", prefix)?;
            }

            return Ok(());
        }

        write!(f, "empty")
    }
}

enum Quantifier {
    All,
    Any,
}

struct SubtypeVisitor<'a> {
    graphs: Pair<&'a Graph>,
    history: BitMatrix,
    visits: Pair<Box<[usize]>>,
}

impl SubtypeVisitor<'_> {
    #[inline]
    fn unroll<I: Iterator<Item = (NodeIndex, Prefix)>, const SWAP: bool>(
        &mut self,
        mut edges: Pair<I>,
        mut prefixes: Pair<Cow<Prefixes>>,
        mut quantifiers: Pair<Quantifier>,
    ) -> bool {
        if SWAP {
            edges = edges.swap();
            prefixes = prefixes.swap();
            quantifiers = quantifiers.swap();
        }

        // TODO: use a write log and snapshots to avoid cloning.
        let (left_prefixes, right_prefixes) = prefixes.as_mut().map(Cow::to_mut).into_tuple();
        let mut right_edges = edges.right.map(|(n, p)| (n, Some(p))).collect::<Vec<_>>();

        let mut output = edges.left.map(|(left_node, left_prefix)| {
            left_prefixes.queue.push_back(left_prefix);
            let mut output = right_edges.iter_mut().map(|(right_node, right_prefix)| {
                right_prefixes.queue.push_back(right_prefix.take().unwrap());
                let mut nodes = Pair::new(left_node, *right_node);
                let mut prefixes = Pair::new(&*left_prefixes, &*right_prefixes).map(Cow::Borrowed);

                if SWAP {
                    nodes = nodes.swap();
                    prefixes = prefixes.swap();
                }

                let output = self.visit(nodes, prefixes);

                *right_prefix = Some(right_prefixes.queue.pop_back().unwrap());
                output
            });

            let output = match quantifiers.right {
                Quantifier::All => output.all(identity),
                Quantifier::Any => output.any(identity),
            };

            left_prefixes.queue.pop_back().unwrap();
            output
        });

        match quantifiers.left {
            Quantifier::All => output.all(identity),
            Quantifier::Any => output.any(identity),
        }
    }

    fn visit(&mut self, nodes: Pair<NodeIndex>, mut prefixes: Pair<Cow<Prefixes>>) -> bool {
        let indexes = nodes.map(|node| node.index());

        let visits = self.visits.as_ref().zip(indexes);
        if visits.into_iter().any(|(v, i)| v[i] == 0) {
            return false;
        }

        if !reduce(&mut prefixes) {
            return false;
        }

        let pairs = self.graphs.zip(nodes);
        let empty_prefixes = prefixes.iter().all(|prefixes| prefixes.queue.is_empty());

        match pairs.map(|(graph, node)| &graph[node]).into_tuple() {
            (Node::End, Node::End) if empty_prefixes => true,
            (
                Node::Choices {
                    role: left_role,
                    direction: left_direction,
                },
                Node::Choices {
                    role: right_role,
                    direction: right_direction,
                },
            ) => {
                let roles = Pair::new(left_role, right_role);
                let directions = Pair::new(left_direction, right_direction);

                let in_history = self.history.get(indexes);
                if in_history && empty_prefixes {
                    return true;
                }

                let edges = pairs.map(|(graph, node)| graph.edges(node));
                let edges = edges
                    .zip(roles.zip(directions))
                    .map(|(edges, (role, direction))| {
                        edges.map(move |edge| {
                            let prefix = Prefix {
                                role: *role,
                                direction: *direction,
                                label: *edge.weight(),
                            };

                            (edge.target(), prefix)
                        })
                    });

                self.history.set(indexes, true);
                for (visits, i) in self.visits.as_mut().zip(indexes).into_iter() {
                    visits[i] -= 1;
                }

                let output = match directions.into_tuple() {
                    (Direction::Send, Direction::Send) => {
                        let quantifiers = Pair::new(Quantifier::All, Quantifier::Any);
                        self.unroll::<_, false>(edges, prefixes, quantifiers)
                    }
                    (Direction::Send, Direction::Receive) => {
                        let quantifiers = Pair::new(Quantifier::All, Quantifier::All);
                        self.unroll::<_, false>(edges, prefixes, quantifiers)
                    }
                    (Direction::Receive, Direction::Send) => {
                        let quantifiers = Pair::new(Quantifier::Any, Quantifier::Any);
                        self.unroll::<_, false>(edges, prefixes, quantifiers)
                    }
                    (Direction::Receive, Direction::Receive) => {
                        let quantifiers = Pair::new(Quantifier::Any, Quantifier::All);
                        self.unroll::<_, true>(edges, prefixes, quantifiers)
                    }
                };

                self.history.set(indexes, in_history);
                for (visits, i) in self.visits.as_mut().zip(indexes).into_iter() {
                    visits[i] += 1;
                }

                output
            }
            _ => false,
        }
    }
}

fn reduce(prefixes: &mut Pair<Cow<Prefixes>>) -> bool {
    fn reorder<R>(left: &Prefix, rights: &Prefixes, reject: R) -> Option<Option<usize>>
    where
        R: Fn(&Prefix, &Prefix) -> bool,
    {
        let mut rights = rights.queue.iter().enumerate();

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

    while let (Some(left), Some(right)) = prefixes.as_ref().map(|p| p.queue.front()).into_tuple() {
        // Fast path to avoid added control flow.
        if left == right {
            for prefix in prefixes.iter_mut() {
                prefix.to_mut().queue.pop_front().unwrap();
            }

            continue;
        }

        // TODO: cache the results of these checks to only search new actions.
        let i = match left.direction {
            Direction::Send => reorder(left, &prefixes.right, |left, right| {
                right.role == left.role && right.direction == Direction::Send
            }),
            Direction::Receive => reorder(left, &prefixes.right, |left, right| {
                right.role == left.role || right.direction == Direction::Send
            }),
        };

        match i {
            Some(Some(i)) => {
                prefixes.left.to_mut().queue.pop_front().unwrap();
                prefixes.right.to_mut().queue.remove(i).unwrap();
                continue;
            }
            Some(None) => break,
            None => return false,
        }
    }

    true
}

pub fn is_subtype(left: &Serialized, right: &Serialized, visits: usize) -> bool {
    assert_eq!(left.role, right.role);

    let sizes = Pair::new(left.graph.node_count(), right.graph.node_count());
    let mut visitor = SubtypeVisitor {
        graphs: Pair::new(&left.graph, &right.graph),
        history: BitMatrix::new(sizes),
        visits: sizes.map(|size| vec![visits; size].into_boxed_slice()),
    };

    visitor.visit(Default::default(), Default::default())
}
