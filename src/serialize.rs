#![cfg(feature = "serialize")]

use crate::{Branch, End, FromState, Receive, Role, Select, Send};
use bitvec::{bitbox, boxed::BitBox};
use fmt::Debug;
use petgraph::{dot::Dot, graph::NodeIndex, visit::EdgeRef};
use std::{
    any::{type_name, TypeId},
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Display, Formatter},
    mem,
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

    fn swap(&mut self) {
        mem::swap(&mut self.left, &mut self.right)
    }

    fn zip<U>(self, other: Pair<U>) -> Pair<(T, U)> {
        Pair::new((self.left, other.left), (self.right, other.right))
    }

    fn map<U>(self, f: impl Fn(T) -> U) -> Pair<U> {
        Pair::new(f(self.left), f(self.right))
    }

    fn into_iter(self) -> impl Iterator<Item = T> {
        self.map(Some)
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

#[derive(Debug, Default)]
struct Prefixes {
    prefixes: Vec<(bool, Prefix)>,
    start: usize,
    removed: Vec<usize>,
}

#[derive(Clone, Copy)]
struct PrefixIndex(usize);

#[derive(Debug, PartialEq, Eq)]
struct PrefixesSnapshot {
    size: usize,
    start: usize,
    removed: usize,
}

impl Prefixes {
    fn is_empty(&self) -> bool {
        self.start >= self.prefixes.len()
    }

    fn first(&self) -> Option<&Prefix> {
        if let Some((removed, prefix)) = self.prefixes.get(self.start) {
            assert!(!removed);
            return Some(&prefix);
        }

        None
    }

    fn push(&mut self, prefix: Prefix) {
        self.prefixes.push((false, prefix));
    }

    fn remove_first(&mut self) {
        assert!(matches!(self.prefixes.get(self.start), Some((false, _))));
        self.start += 1;
        while let Some((true, _)) = self.prefixes.get(self.start) {
            self.start += 1;
        }
    }

    fn remove(&mut self, PrefixIndex(i): PrefixIndex) {
        if i == self.start {
            self.remove_first();
            return;
        }

        let (removed, _) = &mut self.prefixes[i];
        assert!(!*removed);
        *removed = true;
        self.removed.push(i);
    }

    fn snapshot(&self) -> PrefixesSnapshot {
        PrefixesSnapshot {
            size: self.prefixes.len(),
            start: self.start,
            removed: self.removed.len(),
        }
    }

    fn restore(&mut self, snapshot: &PrefixesSnapshot) {
        for &i in self.removed.get(snapshot.removed..).unwrap_or_default() {
            let (removed, _) = &mut self.prefixes[i];
            assert!(*removed);
            *removed = false;
        }

        assert!(snapshot.removed <= self.removed.len());
        self.removed.truncate(snapshot.removed);

        assert!(snapshot.size <= self.prefixes.len());
        self.prefixes.truncate(snapshot.size);

        assert!(snapshot.start <= self.start);
        self.start = snapshot.start;
    }

    fn iter(&self) -> impl Iterator<Item = (PrefixIndex, &Prefix)> {
        let prefixes = self.prefixes.iter().enumerate().skip(self.start);
        prefixes.filter_map(|(i, (removed, prefix))| (!removed).then(|| (PrefixIndex(i), prefix)))
    }
}

impl Display for Prefixes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut prefixes = self.iter().map(|(_, prefix)| prefix);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Quantifier {
    All,
    Any,
}

struct SubtypeVisitor<'a> {
    graphs: Pair<&'a Graph>,
    history: BitMatrix,
    visits: Pair<Box<[usize]>>,
    prefixes: Pair<Prefixes>,
}

impl SubtypeVisitor<'_> {
    #[inline]
    fn unroll<I: Iterator<Item = (NodeIndex, Prefix)>, const SWAP: bool>(
        &mut self,
        mut edges: Pair<I>,
        mut quantifiers: Pair<Quantifier>,
    ) -> bool {
        let mut prefixes = self.prefixes.as_ref();
        if SWAP {
            prefixes.swap();
            edges.swap();
            quantifiers.swap();
        }

        let right_edges = edges.right.collect::<Vec<_>>();
        let snapshots = prefixes.map(Prefixes::snapshot);

        for (left_node, left_prefix) in edges.left {
            let mut prefixes = self.prefixes.as_mut();
            if SWAP {
                prefixes.swap();
            }

            prefixes.left.restore(&snapshots.left);
            prefixes.left.push(left_prefix);

            let mut output = quantifiers.right == Quantifier::All;
            for (right_node, right_prefix) in &right_edges {
                let mut prefixes = self.prefixes.as_mut();
                if SWAP {
                    prefixes.swap();
                }

                prefixes.right.restore(&snapshots.right);
                prefixes.right.push(right_prefix.clone());

                let mut nodes = Pair::new(left_node, *right_node);
                if SWAP {
                    nodes.swap();
                }

                output = self.visit(nodes);

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

    fn visit(&mut self, nodes: Pair<NodeIndex>) -> bool {
        let indexes = nodes.map(|node| node.index());

        let visits = self.visits.as_ref().zip(indexes);
        if visits.into_iter().any(|(v, i)| v[i] == 0) {
            return false;
        }

        if !reduce(&mut self.prefixes) {
            return false;
        }

        let pairs = self.graphs.zip(nodes);
        let empty_prefixes = self.prefixes.iter().all(Prefixes::is_empty);

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
                        self.unroll::<_, false>(edges, quantifiers)
                    }
                    (Direction::Send, Direction::Receive) => {
                        let quantifiers = Pair::new(Quantifier::All, Quantifier::All);
                        self.unroll::<_, false>(edges, quantifiers)
                    }
                    (Direction::Receive, Direction::Send) => {
                        let quantifiers = Pair::new(Quantifier::Any, Quantifier::Any);
                        self.unroll::<_, false>(edges, quantifiers)
                    }
                    (Direction::Receive, Direction::Receive) => {
                        let quantifiers = Pair::new(Quantifier::Any, Quantifier::All);
                        self.unroll::<_, true>(edges, quantifiers)
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

fn reduce(prefixes: &mut Pair<Prefixes>) -> bool {
    fn reorder<R>(left: &Prefix, rights: &Prefixes, reject: R) -> Option<Option<PrefixIndex>>
    where
        R: Fn(&Prefix, &Prefix) -> bool,
    {
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

    while let (Some(left), Some(right)) = prefixes.as_ref().map(Prefixes::first).into_tuple() {
        // Fast path to avoid added control flow.
        if left == right {
            for prefixes in prefixes.iter_mut() {
                prefixes.remove_first();
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

pub fn is_subtype(left: &Serialized, right: &Serialized, visits: usize) -> bool {
    assert_eq!(left.role, right.role);

    let sizes = Pair::new(left.graph.node_count(), right.graph.node_count());
    let mut visitor = SubtypeVisitor {
        graphs: Pair::new(&left.graph, &right.graph),
        history: BitMatrix::new(sizes),
        visits: sizes.map(|size| vec![visits; size].into_boxed_slice()),
        prefixes: Default::default(),
    };

    visitor.visit(Default::default())
}
