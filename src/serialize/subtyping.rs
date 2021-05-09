use super::{bit_matrix::BitMatrix, pair::Pair, Direction, Graph, Label, Node, Serialized, Type};
use petgraph::{graph::NodeIndex, visit::EdgeRef};
use std::fmt::{self, Display, Formatter};

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

        match pairs.map(|(graph, node)| &graph[node]).into() {
            (Node::End, Node::End) if empty_prefixes => true,
            (Node::Choices(left_choices), Node::Choices(right_choices)) => {
                let choices = Pair::new(left_choices, right_choices);

                let in_history = self.history.get(indexes);
                if in_history && empty_prefixes {
                    return true;
                }

                let edges = pairs.map(|(graph, node)| graph.edges(node));
                let edges = edges.zip(choices).map(|(edges, choices)| {
                    edges.map(move |edge| {
                        let prefix = Prefix {
                            role: choices.role,
                            direction: choices.direction,
                            label: *edge.weight(),
                        };

                        (edge.target(), prefix)
                    })
                });

                self.history.set(indexes, true);
                for (visits, i) in self.visits.as_mut().zip(indexes).into_iter() {
                    visits[i] -= 1;
                }

                let output = match choices.map(|choices| choices.direction).into() {
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

    while let (Some(left), Some(right)) = prefixes.as_ref().map(Prefixes::first).into() {
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
