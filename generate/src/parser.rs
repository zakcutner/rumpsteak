#![allow(clippy::upper_case_acronyms)]

use super::{Graph, GraphEdge, GraphNode, Result};
use indexmap::{IndexMap, IndexSet};
use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    convert::TryFrom,
};

extern crate dot_parser;
use dot_parser::ast::{
    Graph as DotGraph,
    Stmt,
    NodeStmt,
    NodeID,
};

struct Context<'a> {
    roles: IndexSet<&'a str>,
    labels: IndexMap<&'a str, Vec<&'a str>>,
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
    fn try_from(tuple: (DotGraph<'a, label::Label<'a>>, &mut Context<'a>)) -> Result<Self, Self::Error> {
        let (value, context) = tuple;
        let mut nodes: Vec<Node<'a>> = Vec::new();
        let mut edges = Vec::new();

        if let Err(()) = check_graph_edges(&value) {
            return Err(());
        }

        for statement in value.stmts {
            match statement {
                Stmt::NodeStmt(node) => { nodes.push(node.into()) },
                Stmt::EdgeStmt(edge) => { edges.push(edge) },
                _ => { /* Ignore AttrStmt and IDEq */ },
            }
        }

        let mut graph = Graph::with_capacity(nodes.len(), edges.len());

        let node_indexes = nodes
            .into_iter()
            .map(|node| (node.name, graph.add_node(GraphNode::new(node.name))))
            .collect::<HashMap<_, _>>();

        for edge in edges {
            let attr = edge.attr.unwrap().elems[0].elems[0].clone();
            let (role, direction, payload, params) = attr.fields();

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
            let edge = GraphEdge::new(payload_index, None);
            graph.add_edge(from_index, to_index, edge);
        }

        Ok(Digraph { graph }) 
    }
}

#[derive(Debug)]
pub(crate) struct Tree<'a> {
    pub roles: Vec<(&'a str, Graph<'a>)>,
    pub labels: IndexMap<&'a str, Vec<&'a str>>,
}

impl<'a> Tree<'a> {
    pub fn parse(inputs: &'a [String]) -> Result<Self> {
        let mut context = Context::with_capacity(inputs.len());
        let roles = inputs.iter().map(|input| {
            let dot_graph = DotGraph::read_dot(input)
                .unwrap()
                .filter_map(|(key, value)| 
                    if key == "label" {
                        let label = label::Label::from_str(value).unwrap();
                        Some(label)
                    } else {
                        None
                    });
            let role = dot_graph.name.unwrap(); // panic if the graph is not named
            if !context.roles.insert(role) {
                let message = "duplicate graphs found for role";
                return Err(error_msg(message.to_owned()));
            }

            Ok((role, dot_graph))
        });

        let roles = roles
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|(name, dot_graph)| {
                //TODO: handle error properly if try_from fails
                Ok((name, Digraph::try_from((dot_graph, &mut context)).unwrap().graph))
            });

        Ok(Tree {
            roles: roles.collect::<Result<Vec<_>>>()?,
            labels: context.labels,
        })
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
    Box::new(StrError{ msg: message })
}

#[derive(Copy, Clone)]
/// The direction of the state. The &str carried contains the name of the receiving/sending peer.
enum NodeDirection<'a> {
    Unspecified,
    Send(&'a str),
    Receive(&'a str)
}

fn check_graph_edges<'a>(graph: &DotGraph<'a, label::Label<'a>>) -> Result<(), ()> {
    let mut nodes: IndexMap<&str, NodeDirection> = (&graph.stmts).into_iter().filter_map(|stmt| stmt.get_node_ref()).map(|n| (n.name(), NodeDirection::Unspecified)).collect();
    let mut payload_types: IndexMap<&str, &Vec<&str>> = IndexMap::new();

    for edge in (&graph.stmts).into_iter().filter_map(|e| e.get_edge_ref()) {
        let from = edge.node.id;
        let to = edge.next.node.id;
        if let Some(_) = edge.next.next {
            eprintln!("Chaining multiple edges at once is not supported: split the chain into individual edges.");
            return Err(());
        }
        let edge_label: Option<&label::Label<'a>> = edge.attr.as_ref().map(|attr| attr.flatten_ref().elems.pop()).flatten();
        let edge_direction_role = edge_label.as_ref().map(|label| (label.direction, label.role));
        let edge_payload = edge_label.as_ref().map(|label| (label.payload, &label.parameters));

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
                    },
                    None => {
                        payload_types.insert(payload, parameters);
                    }
                }
            },
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
    use pest::Parser;
    use pest_derive::Parser;
    use pest::iterators::Pair;
    use std::convert::{TryFrom, TryInto};

#[derive(Parser)]
#[grammar = "parser/label.pest"]
    pub (in crate) struct LabelParser;

    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone)]
    pub (in crate) enum Direction {
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
    pub struct Label<'a> {
        pub (in crate) role: &'a str,
        pub (in crate) direction: Direction, 
        pub (in crate) payload: &'a str,
        pub (in crate) parameters: Vec<&'a str>,
    }

    impl<'a> Label<'a> {
        pub (in crate) fn parse(p: Pair<'a, Rule>) -> Result<Self, ()> {
            if let Rule::label = p.as_rule() {
                let mut inner = p.into_inner();
                let role = inner.next().unwrap().as_str();
                let direction = inner.next().unwrap().as_rule().try_into().unwrap(); 
                let payload = inner.next().unwrap().as_str();
                let mut parameters = Vec::new();
                for pair in inner {
                    if pair.as_str() != "" {
                        parameters.push(pair.as_str());
                    }
                }
                Ok(Label { role, direction, payload, parameters })
            } else {
                Err(())
            }
        }

        pub (in crate) fn from_str(s: &'a str) -> Result<Self, ()> {
            let label = LabelParser::parse(Rule::label, s);
            if let Err(e) = &label {
                println!("{}", e);
            } 
            Label::parse(label.unwrap().next().unwrap())
        }
        
        pub (in crate) fn fields(self) -> (&'a str, Direction, &'a str, Vec<&'a str>) {
            (self.role, self.direction, self.payload, self.parameters)
        }
    }
}
