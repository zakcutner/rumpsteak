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

        eprintln!("{:#?}", tree);

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
            eprintln!("cannot send to or receive from own role");
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
    pub(in crate) enum Predicate<'a> {
        LTnVar(&'a str, &'a str),
        LTnConst(&'a str, &'a str),
        GTnVar(&'a str, &'a str),
        GTnConst(&'a str, &'a str),
        EqualVar(&'a str, &'a str),
        EqualConst(&'a str, &'a str),
        LTnThree(&'a str, &'a str, &'a str),
        GTnThree(&'a str, &'a str, &'a str),
        None,
    }

    impl<'a> Into<super::super::Predicate> for Predicate<'a> {
        fn into(self) -> super::super::Predicate {
            match self {
                Predicate::LTnVar(a, b) => {
                    super::super::Predicate::LTnVar(a.to_string(), b.to_string())
                }
                Predicate::LTnConst(a, b) => {
                    super::super::Predicate::LTnConst(a.to_string(), b.to_string())
                }
                Predicate::GTnVar(a, b) => {
                    super::super::Predicate::GTnVar(a.to_string(), b.to_string())
                }
                Predicate::GTnConst(a, b) => {
                    super::super::Predicate::GTnConst(a.to_string(), b.to_string())
                }
                Predicate::EqualVar(a, b) => {
                    super::super::Predicate::EqualVar(a.to_string(), b.to_string())
                }
                Predicate::EqualConst(a, b) => {
                    super::super::Predicate::EqualConst(a.to_string(), b.to_string())
                }
                Predicate::LTnThree(a, b, c) => {
                    super::super::Predicate::LTnThree(a.to_string(), b.to_string(), c.to_string())
                }
                Predicate::GTnThree(a, b, c) => {
                    super::super::Predicate::GTnThree(a.to_string(), b.to_string(), c.to_string())
                }
                Predicate::None => super::super::Predicate::None,
            }
        }
    }

    impl<'a> Predicate<'a> {
        pub(in crate) fn parse(p: Pair<'a, Rule>) -> Result<Self, ()> {
            match p.as_rule() {
                Rule::op => {
                    let mut inner = p.clone().into_inner();
                    let param = inner.next().unwrap().as_str();
                    let op = inner.next().unwrap();
                    let value = inner.next().unwrap().as_str();
                    let first_char = value.chars().nth(0).unwrap();
                    match op.as_rule() {
                        Rule::ltn => {
                            // check is the second operand is a number
                            if first_char.is_digit(10) {
                                Ok(Predicate::LTnConst(param, value))
                            } else {
                                Ok(Predicate::LTnVar(param, value))
                            }
                        }
                        Rule::gtn => {
                            // check is the second operand is a number
                            if first_char.is_digit(10) {
                                Ok(Predicate::GTnConst(param, value))
                            } else {
                                Ok(Predicate::GTnVar(param, value))
                            }
                        }
                        Rule::eq => {
                            // check is the second operand is a number
                            if first_char.is_digit(10) {
                                Ok(Predicate::EqualConst(param, value))
                            } else {
                                Ok(Predicate::EqualVar(param, value))
                            }
                        }
                        _ => Err(()),
                    }
                }
                Rule::compthree => {
                    let mut inner = p.clone().into_inner();
                    let param = inner.next().unwrap().as_str();
                    let op = inner.next().unwrap();
                    let value1 = inner.next().unwrap().as_str();
                    let _ = inner.next().unwrap();
                    let value2 = inner.next().unwrap().as_str();
                    match op.as_rule() {
                        Rule::ltn => Ok(Predicate::LTnThree(param, value1, value2)),
                        Rule::gtn => Ok(Predicate::GTnThree(param, value1, value2)),
                        _ => Err(()),
                    }
                }
                _ => Err(()),
            }
        }
    }

    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
    pub(in crate) enum BoolPredicate<'a> {
        Normal(Predicate<'a>),
        And(Predicate<'a>, Predicate<'a>),
        Or(Predicate<'a>, Predicate<'a>),
        Neg(Predicate<'a>),
        None,
    }

    impl<'a> BoolPredicate<'a> {
        pub(in crate) fn parse(p: Pair<'a, Rule>) -> Result<Self, ()> {
            match p.as_rule() {
                Rule::basic => {
                    let mut inner = p.clone().into_inner();
                    let pred = if let Some(p) = inner.next() {
                        Predicate::parse(p).unwrap()
                    } else {
                        Predicate::None
                    };
                    return Ok(BoolPredicate::Normal(pred));
                }
                Rule::bool_op => {
                    let mut inner = p.clone().into_inner();
                    let pred1 = if let Some(p) = inner.next() {
                        Predicate::parse(p).unwrap()
                    } else {
                        Predicate::None
                    };

                    let op = inner.next().unwrap();
                    let pred2 = if let Some(p) = inner.next() {
                        Predicate::parse(p).unwrap()
                    } else {
                        Predicate::None
                    };
                    return match op.as_rule() {
                        Rule::and => Ok(BoolPredicate::And(pred1, pred2)),
                        Rule::or => Ok(BoolPredicate::Or(pred1, pred2)),
                        _ => Err(()),
                    };
                }
                Rule::neg_op => {
                    let mut inner = p.clone().into_inner();
                    let op = inner.next().unwrap();
                    let pred = if let Some(p) = inner.next() {
                        Predicate::parse(p).unwrap()
                    } else {
                        Predicate::None
                    };
                    return match op.as_rule() {
                        Rule::neg => Ok(BoolPredicate::Neg(pred)),
                        _ => Err(()),
                    };
                }
                _ => return Err(()),
            }
        }
    }

    impl<'a> Into<super::super::BoolPredicate> for BoolPredicate<'a> {
        fn into(self) -> super::super::BoolPredicate {
            match self {
                BoolPredicate::Normal(pred) => super::super::BoolPredicate::Normal(pred.into()),
                BoolPredicate::And(pred1, pred2) => {
                    super::super::BoolPredicate::And(pred1.into(), pred2.into())
                }
                BoolPredicate::Or(pred1, pred2) => {
                    super::super::BoolPredicate::Or(pred1.into(), pred2.into())
                }
                BoolPredicate::Neg(pred) => super::super::BoolPredicate::Neg(pred.into()),
                _ => super::super::BoolPredicate::None,
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
        pub(in crate) predicate: BoolPredicate<'a>,
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
                let mut predicate = BoolPredicate::None;
                let mut side_effect = SideEffect::None;
                while let Some(p) = inner.next() {
                    match p.as_rule() {
                        Rule::predicate => {
                            if let Some(p) = p.into_inner().next() {
                                predicate = BoolPredicate::parse(p).unwrap();
                                eprintln!("{:#?}", predicate);
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
            BoolPredicate<'a>,
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
