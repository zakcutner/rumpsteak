#![allow(clippy::upper_case_acronyms)]

use super::{Graph, GraphEdge, GraphNode, Result};
use indexmap::{IndexMap, IndexSet};
use std::{
    collections::HashMap,
    convert::TryFrom,
    error::Error,
    fmt::{self, Display, Formatter},
};

extern crate dot_parser;
use dot_parser::ast::{Graph as DotGraph, NodeID, NodeStmt, Stmt};

struct Context<'a> {
    roles: IndexSet<&'a str>,
    labels: IndexMap<&'a str, Vec<(&'a str, &'a str)>>,
}

impl<'a> Context<'a> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            roles: IndexSet::with_capacity(capacity),
            labels: IndexMap::new(),
        }
    }
}

struct Node<'a> {
    name: &'a str,
}

impl<'a> From<NodeStmt<'a, label::Label<'a>>> for Node<'a> {
    fn from(node: NodeStmt<'a, label::Label<'a>>) -> Node<'a> {
        node.node.into()
    }
}

impl<'a> From<NodeID<'a>> for Node<'a> {
    fn from(node: NodeID<'a>) -> Node<'a> {
        Node { name: node.id }
    }
}

struct Digraph<'a> {
    graph: Graph<'a>,
}

impl<'a> TryFrom<(DotGraph<'a, label::Label<'a>>, &mut Context<'a>)> for Digraph<'a> {
    type Error = ();
    fn try_from(
        tuple: (DotGraph<'a, label::Label<'a>>, &mut Context<'a>),
    ) -> Result<Self, Self::Error> {
        let (value, context) = tuple;
        let mut nodes: Vec<Node<'a>> = Vec::new();
        let mut edges: Vec<dot_parser::ast::EdgeStmt<'a, label::Label<'a>>> = Vec::new();

        if let Err(()) = check_graph_edges(&value) {
            return Err(());
        }

        for statement in value.stmts {
            match statement {
                Stmt::NodeStmt(node) => nodes.push(node.into()),
                Stmt::EdgeStmt(edge) => edges.push(edge),
                _ => { /* Ignore AttrStmt and IDEq */ }
            }
        }

        let mut graph = Graph::with_capacity(nodes.len(), edges.len());

        let node_indexes = nodes
            .into_iter()
            .map(|node| (node.name, graph.add_node(GraphNode::new(node.name))))
            .collect::<HashMap<_, _>>();

        for edge in edges {
            let attr = edge.attr.unwrap().elems[0].elems[0].clone();
            let (role, direction, payload, params, predicate, side_effect) = attr.fields();

            if !context.labels.contains_key(payload) {
                context.labels.insert(payload, params);
            }
            // Always succeed since we just inserted the key
            let payload_index = context.labels.get_index_of(payload).unwrap();
            let role_index = context.roles.get_index_of(role).unwrap();

            let from = edge.node.id;
            let to = edge.next.node.id;
            let from_index = node_indexes[from];
            let to_index = node_indexes[to];
            graph[from_index].direction = Some(direction.into());
            graph[from_index].role = Some(role_index);
            let edge = GraphEdge::new(payload_index, predicate.into(), side_effect.into());
            graph.add_edge(from_index, to_index, edge);
        }

        Ok(Digraph { graph })
    }
}

#[derive(Debug)]
pub(crate) struct Tree<'a> {
    pub roles: Vec<(&'a str, Graph<'a>)>,
    pub labels: IndexMap<&'a str, Vec<(&'a str, &'a str)>>,
}

impl<'a> Tree<'a> {
    pub fn parse(inputs: &'a [String]) -> Result<Self> {
        let mut context = Context::with_capacity(inputs.len());
        let roles = inputs.iter().map(|input| {
            let dot_graph = match DotGraph::read_dot(input) {
                Ok(graph) => graph.filter_map(|(key, value)| {
                    if key == "label" {
                        let label = label::Label::from_str(value).unwrap();
                        Some(label)
                    } else {
                        None
                    }
                }),
                Err(e) => {
                    eprintln!("{}", e);
                    panic!();
                }
            };
            let role = dot_graph.name.unwrap(); // panic if the graph is not named
            if !context.roles.insert(role) {
                let message = format!("Duplicate graphs found for role {}", role);
                return Err(error_msg(message.to_owned()));
            }

            Ok((role, dot_graph))
        });

        let roles = roles
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|(name, dot_graph)| {
                //TODO: handle error properly if try_from fails
                Ok((
                    name,
                    Digraph::try_from((dot_graph, &mut context)).unwrap().graph,
                ))
            });

        let tree = Tree {
            roles: roles.collect::<Result<Vec<_>>>()?,
            labels: context.labels,
        };

        Ok(tree)
    }
}

#[derive(Debug)]
struct StrError {
    msg: String,
}

impl Display for StrError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Error for StrError {}

fn error_msg(message: String) -> Box<dyn Error> {
    Box::new(StrError { msg: message })
}

#[derive(Copy, Clone)]
/// The direction of the state. The &str carried contains the name of the receiving/sending peer.
enum NodeDirection<'a> {
    Unspecified,
    Send(&'a str),
    Receive(&'a str),
}

fn check_graph_edges<'a>(graph: &DotGraph<'a, label::Label<'a>>) -> Result<(), ()> {
    let mut nodes: IndexMap<&str, NodeDirection> = (&graph.stmts)
        .into_iter()
        .filter_map(|stmt| stmt.get_node_ref())
        .map(|n| (n.name(), NodeDirection::Unspecified))
        .collect();
    let mut payload_types: IndexMap<&str, &Vec<(&str, &str)>> = IndexMap::new();

    for edge in (&graph.stmts).into_iter().filter_map(|e| e.get_edge_ref()) {
        let from = edge.node.id;
        let to = edge.next.node.id;
        if let Some(_) = edge.next.next {
            eprintln!("Chaining multiple edges at once is not supported: split the chain into individual edges.");
            return Err(());
        }
        let edge_label: Option<&label::Label<'a>> = edge
            .attr
            .as_ref()
            .map(|attr| attr.flatten_ref().elems.pop())
            .flatten();
        let edge_direction_role = edge_label
            .as_ref()
            .map(|label| (label.direction, label.role));
        let edge_payload = edge_label
            .as_ref()
            .map(|label| (label.payload, &label.parameters));

        match edge_payload {
            Some((payload, parameters)) => {
                let supposed_parameters = payload_types.get(payload);
                match supposed_parameters {
                    Some(param) => {
                        if &parameters != param {
                            eprintln!(
                                "label was previously used with different parameters `{}`",
                                payload,
                            );
                            return Err(());
                        }
                    }
                    None => {
                        payload_types.insert(payload, parameters);
                    }
                }
            }
            None => {
                eprintln!("An edge has no payload");
                return Err(());
            }
        }

        let node_direction = match nodes.get(from) {
            Some(e) => e,
            None => {
                eprintln!("A node is used but not declared");
                return Err(());
            }
        };

        match edge_direction_role {
            Some((label::Direction::Send, role)) => {
                match node_direction {
                    NodeDirection::Receive(_) => {
                        eprintln!("all outgoing transitions must either send to or receive from the same role 1");
                        return Err(());
                    }
                    NodeDirection::Send(peer) => {
                        if peer != &role {
                            eprintln!("all outgoing transitions must either send to or receive from the same role (found {}, expected {})", peer, to);
                            return Err(());
                        }
                    }
                    _ => {}
                }
                nodes.insert(from, NodeDirection::Send(role));
            }
            Some((label::Direction::Receive, role)) => {
                match node_direction {
                    NodeDirection::Send(_) => {
                        eprintln!("all outgoing transitions must either send to or receive from the same role 3");
                        return Err(());
                    }
                    NodeDirection::Receive(peer) => {
                        if peer != &role {
                            eprintln!("all outgoing transitions must either send to or receive from the same role (found {}, expected {})", peer, to);
                            return Err(());
                        }
                    }
                    _ => {}
                }
                nodes.insert(from, NodeDirection::Receive(role));
            }
            None => {
                eprintln!("An edge has no correct label");
                return Err(());
            }
        }

        if graph.name == edge_label.map(|label| label.role) {
            eprintln!("cannot send to or receive from own role (role {:?})", graph.name);
            return Err(());
        }
    }
    Ok(())
}

mod label {
    use pest::iterators::Pair;
    use pest::Parser;
    use pest_derive::Parser;
    use std::convert::{TryFrom, TryInto};

    #[derive(Parser)]
    #[grammar = "parser/label.pest"]
    pub(in crate) struct LabelParser;

    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone)]
    pub(in crate) enum Direction {
        Send,
        Receive,
    }

    impl<'a> TryFrom<Rule> for Direction {
        type Error = ();
        fn try_from(value: Rule) -> Result<Self, Self::Error> {
            match value {
                Rule::send => Ok(Direction::Send),
                Rule::receive => Ok(Direction::Receive),
                _ => Err(()),
            }
        }
    }

    impl Into<super::super::Direction> for Direction {
        fn into(self) -> super::super::Direction {
            match self {
                Direction::Send => super::super::Direction::Send,
                Direction::Receive => super::super::Direction::Receive,
            }
        }
    }

    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
    pub(in crate) enum Atom {
        Var(String),
        Const(String),
    }

    impl Atom {
        pub (in crate) fn parse<'a>(p: Pair<'a, Rule>) -> Result<Self, ()> {
            match p.as_rule() {
                Rule::variable => {
                    let name: String = p.as_str().to_string();
                    Ok(Atom::Var(name))
                }
                Rule::constant => {
                    let val: String = p.as_str().to_string();
                    Ok(Atom::Const(val))
                }
                _ => {
                    Err(())
                }
            }
        }
    }

    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
    pub(in crate) enum Predicate {
        LTn(Atom, Atom),
        GTn(Atom, Atom),
        Equal(Atom, Atom),
    }

    impl Into<super::super::Predicate> for Predicate {
        fn into(self) -> super::super::Predicate {
            match self {
                Predicate::LTn(a, b) => {
                    match a {
                        Atom::Var(a_name) => {
                            match b {
                                Atom::Var(b_name) => {
                                    super::super::Predicate::LTnVar(a_name, b_name, None)
                                }
                                Atom::Const(val) => {
                                    super::super::Predicate::LTnConst(a_name, val, None)
                                }
                            }
                        }
                        Atom::Const(val) => {
                            panic!()
                        }
                    }
                }
                Predicate::GTn(a, b) => {
                    match a {
                        Atom::Var(a_name) => {
                            match b {
                                Atom::Var(b_name) => {
                                    super::super::Predicate::GTnVar(a_name, b_name, None)
                                }
                                Atom::Const(val) => {
                                    super::super::Predicate::GTnConst(a_name, val, None)
                                }
                            }
                        }
                        Atom::Const(val) => {
                            panic!()
                        }
                    }
                }
                Predicate::Equal(a, b) => {
                    match a {
                        Atom::Var(a_name) => {
                            match b {
                                Atom::Var(b_name) => {
                                    super::super::Predicate::EqualVar(a_name, b_name, None)
                                }
                                Atom::Const(val) => {
                                    super::super::Predicate::EqualConst(a_name, val, None)
                                }
                            }
                        }
                        Atom::Const(val) => {
                            panic!()
                        }
                    }
                }
            }
        }
    }

    impl Predicate {
        pub(in crate) fn parse<'a>(p: Pair<'a, Rule>) -> Result<Self, ()> {
            if let Rule::comp = p.as_rule() {
                let mut inner = p.into_inner();
                let lhs: Atom = Atom::parse(inner.next().unwrap())?;
                let op = inner.next().unwrap();
                let rhs: Atom = Atom::parse(inner.next().unwrap())?;

                match op.as_rule() {
                    Rule::ltn => {
                        Ok(Predicate::LTn(lhs, rhs))
                    }
                    Rule::gtn => {
                            Ok(Predicate::GTn(lhs, rhs))
                    }
                    Rule::eq => {
                            Ok(Predicate::Equal(lhs, rhs))
                    }
                    _ => Err(()),
                }

            } else {
                Err(())
            }
        }
    }

    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
    pub(in crate) enum BoolPredicate {
        Normal(Predicate),
        And(Box<BoolPredicate>, Box<BoolPredicate>),
        Or(Box<BoolPredicate>, Box<BoolPredicate>),
        Neg(Box<BoolPredicate>),
        Tautology,
    }

    impl BoolPredicate {
        pub(in crate) fn parse<'a>(p: Pair<'a, Rule>) -> Result<Self, ()> {
            match p.as_rule() {
                Rule::comp => {
                    let res = BoolPredicate::Normal(Predicate::parse(p)?);
                    Ok(res)
                }
                Rule::neg => {
                    let mut inners = p.into_inner();
                    let inner = inners.next().unwrap();
                    let inner_rule = inner.as_rule();
                    if let Rule::predicate = inner_rule {
                        let mut inners = inner.into_inner();
                        let inner = inners.next().unwrap();
                        let inner = BoolPredicate::parse(inner)?;
                        Ok(BoolPredicate::Neg(Box::new(inner)))
                    } else {
                        Err(())
                    }
                }
                Rule::or => {
                    let mut inners = p.into_inner();
                    let lhs = inners.next().unwrap();
                    let lhs_rule = lhs.as_rule();
                    let rhs = inners.next().unwrap();
                    let rhs_rule = rhs.as_rule();
                    if Rule::predicate == rhs_rule && Rule::predicate == lhs_rule {
                        let mut rhs_inners = rhs.into_inner();
                        let rhs = rhs_inners.next().unwrap();
                        let rhs = BoolPredicate::parse(rhs)?;

                        let mut lhs_inners = lhs.into_inner();
                        let lhs = lhs_inners.next().unwrap();
                        let lhs = BoolPredicate::parse(lhs)?;
                        Ok(BoolPredicate::Or(Box::new(rhs), Box::new(lhs)))
                    } else {
                        Err(())
                    }
                }
                Rule::and => {
                    let mut inners = p.into_inner();
                    let lhs = inners.next().unwrap();
                    let lhs_rule = lhs.as_rule();
                    let rhs = inners.next().unwrap();
                    let rhs_rule = rhs.as_rule();
                    if Rule::predicate == rhs_rule && Rule::predicate == lhs_rule {
                        let mut rhs_inners = rhs.into_inner();
                        let rhs = rhs_inners.next().unwrap();
                        let rhs = BoolPredicate::parse(rhs)?;

                        let mut lhs_inners = lhs.into_inner();
                        let lhs = lhs_inners.next().unwrap();
                        let lhs = BoolPredicate::parse(lhs)?;
                        Ok(BoolPredicate::And(Box::new(rhs), Box::new(lhs)))
                    } else {
                        Err(())
                    }
                }
                _ => {
                    println!("{:#?}", p);
                    return Err(())
                },
            }
        }
    }

    impl Into<super::super::BoolPredicate> for BoolPredicate {
        fn into(self) -> super::super::BoolPredicate {
            match self {
                BoolPredicate::Normal(pred) => super::super::BoolPredicate::Normal(pred.into()),
                BoolPredicate::And(pred1, pred2) => {
                    let lhs: BoolPredicate = *pred1;
                    let rhs: BoolPredicate = *pred2;
                    super::super::BoolPredicate::And(None, Box::new(lhs.into()), Box::new(rhs.into()))
                }
                BoolPredicate::Or(pred1, pred2) => {
                    let lhs: BoolPredicate = *pred1;
                    let rhs: BoolPredicate = *pred2;
                    super::super::BoolPredicate::Or(None, Box::new(lhs.into()), Box::new(rhs.into()))
                }
                BoolPredicate::Neg(pred) => {
                    let inner: BoolPredicate = *pred;
                    super::super::BoolPredicate::Neg(None, Box::new(inner.into()))
                },
                BoolPredicate::Tautology => super::super::BoolPredicate::Normal(super::super::Predicate::Tautology(None)),
            }
        }
    }

    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
    pub(in crate) enum SideEffect<'a> {
        Increase(&'a str, &'a str),
        Decrease(&'a str, &'a str),
        Multiply(&'a str, &'a str),
        Divide(&'a str, &'a str),
        None,
    }

    impl<'a> SideEffect<'a> {
        pub(in crate) fn parse(p: Pair<'a, Rule>) -> Result<Self, ()> {
            let mut inner = p.clone().into_inner();
            let param1 = inner.next().unwrap().as_str();
            let param2 = inner.next().unwrap().as_str();
            assert_eq!(param1, param2);
            let op = inner.next().unwrap();
            let value = inner.next().unwrap().as_str();
            match op.as_rule() {
                Rule::incr => Ok(SideEffect::Increase(param1, value)),
                Rule::decr => Ok(SideEffect::Decrease(param1, value)),
                Rule::mult => Ok(SideEffect::Multiply(param1, value)),
                Rule::div => Ok(SideEffect::Divide(param1, value)),
                _ => Err(()),
            }
        }
    }

    impl<'a> Into<super::super::SideEffect> for SideEffect<'a> {
        fn into(self) -> super::super::SideEffect {
            match self {
                SideEffect::Increase(a, b) => {
                    super::super::SideEffect::Increase(a.to_string(), b.to_string())
                }
                SideEffect::Decrease(a, b) => {
                    super::super::SideEffect::Decrease(a.to_string(), b.to_string())
                }
                SideEffect::Multiply(a, b) => {
                    super::super::SideEffect::Multiply(a.to_string(), b.to_string())
                }
                SideEffect::Divide(a, b) => {
                    super::super::SideEffect::Divide(a.to_string(), b.to_string())
                }
                SideEffect::None => super::super::SideEffect::None,
            }
        }
    }

    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
    pub struct Label<'a> {
        pub(in crate) role: &'a str,
        pub(in crate) direction: Direction,
        pub(in crate) payload: &'a str,
        pub(in crate) parameters: Vec<(&'a str, &'a str)>, // (name, type)
        pub(in crate) predicate: BoolPredicate,
        pub(in crate) side_effect: SideEffect<'a>,
    }

    impl<'a> Label<'a> {
        pub(in crate) fn parse(p: Pair<'a, Rule>) -> Result<Self, ()> {
            if let Rule::label = p.as_rule() {
                let mut inner = p.into_inner();
                let role = inner.next().unwrap().as_str();
                let direction = inner.next().unwrap().as_rule().try_into().unwrap();
                let payload = inner.next().unwrap().as_str();
                let mut parameters = Vec::new();
                let params = inner.next();
                for pair in params {
                    if pair.as_str() != "" {
                        let inner = pair.into_inner();
                        for param in inner {
                            let mut inner = param.into_inner();
                            let name = inner.next().unwrap().as_str();
                            let typ = inner.next().unwrap().as_str();
                            parameters.push((name, typ));
                        }
                    }
                }
                let mut predicate = BoolPredicate::Tautology;
                let mut side_effect = SideEffect::None;
                while let Some(p) = inner.next() {
                    match p.as_rule() {
                        Rule::predicate => {
                            if let Some(p) = p.into_inner().next() {
                                predicate = BoolPredicate::parse(p).unwrap();
                            }
                        }
                        Rule::side_effect => {
                            side_effect = SideEffect::parse(p).unwrap();
                        }
                        _ => (),
                    }
                }
                Ok(Label {
                    role,
                    direction,
                    payload,
                    parameters,
                    predicate,
                    side_effect,
                })
            } else {
                Err(())
            }
        }

        pub(in crate) fn from_str(s: &'a str) -> Result<Self, ()> {
            let label = LabelParser::parse(Rule::label, s);
            if let Err(e) = &label {
                eprintln!("{}", e);
            }
            Label::parse(label.unwrap().next().unwrap())
        }

        pub(in crate) fn fields(
            self,
        ) -> (
            &'a str,
            Direction,
            &'a str,
            Vec<(&'a str, &'a str)>,
            BoolPredicate,
            SideEffect<'a>,
        ) {
            (
                self.role,
                self.direction,
                self.payload,
                self.parameters,
                self.predicate,
                self.side_effect,
            )
        }
    }
}
