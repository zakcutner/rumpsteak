#![allow(clippy::upper_case_acronyms)]

use super::{Direction, Graph, GraphEdge, GraphNode, Result};
use indexmap::{map::Entry, IndexMap, IndexSet};
use pest::{
    error::{Error as PestError, ErrorVariant as PestErrorVariant},
    Parser as _, Span,
};
use pest_derive::Parser;
use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    convert::TryFrom,
};

extern crate dot_parser;
use dot_parser::{
    Graph as DotGraph,
    Stmt,
    NodeStmt,
    NodeID,
};

type Pairs<'i> = pest::iterators::Pairs<'i, Rule>;
type Pair<'i> = pest::iterators::Pair<'i, Rule>;

#[derive(Parser)]
#[grammar = "parser/digraph.pest"]
struct Parser;

#[derive(Debug, PartialEq, Eq)]
struct Parameters<'a>(&'a [&'a str]);

impl Display for Parameters<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut parameters = self.0.iter();
        if let Some(parameter) = parameters.next() {
            write!(f, "{}", parameter)?;
            for parameter in parameters {
                write!(f, ", {}", parameter)?;
            }
        }

        Ok(())
    }
}

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

impl<'a> Node<'a> {
    fn parse(statement: Pair<'a>) -> Self {
        let mut pairs = statement.into_inner();
        Self {
            name: next_pair(&mut pairs, Rule::ident).unwrap().as_str(),
        }
    }
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

struct Edge<'a> {
    from: &'a str,
    to: &'a str,
    role: (usize, Span<'a>),
    direction: (Direction, Span<'a>),
    edge: GraphEdge<'a>,
}

impl<'a> Edge<'a> {
    fn parse(context: &mut Context<'a>, name: &str, statement: Pair<'a>) -> Result<Self> {
        let mut pairs = statement.into_inner();

        //                 direction
        //                 V
        // 1 -> 2 [label="S!HELLO(i32)", ];
        // ^    ^                 ^^^
        // from to          ^^^^^ parameters
        //                ^ label
        //                role
        
        // Initial and destination local state
        let from = next_pair(&mut pairs, Rule::ident).unwrap().as_str();
        let to = next_pair(&mut pairs, Rule::ident).unwrap().as_str();

        // pairs contains the label specification
        let mut pairs = next_pair(&mut pairs, Rule::label).unwrap().into_inner();

        let role = next_pair(&mut pairs, Rule::ident).unwrap();
        if role.as_str() == name {
            let message = "cannot send to or receive from own role";
            return Err(error(role.as_span(), message.to_owned()));
        }

        let i = context.roles.get_index_of(role.as_str());
        let role = (
            i.ok_or_else(|| error(role.as_span(), "unknown role name".to_owned()))?,
            role.as_span(),
        );

        let direction = pairs.next().unwrap();
        let direction = (
            match direction.as_rule() {
                Rule::send => Direction::Send,
                Rule::receive => Direction::Receive,
                _ => unreachable!(),
            },
            direction.as_span(),
        );

        let label = next_pair(&mut pairs, Rule::ident).unwrap();
        let parameters = next_pair(&mut pairs, Rule::parameters).unwrap();
        let parameters = parameters.into_inner().map(|p| p.as_str()).collect();

        let label = match context.labels.entry(label.as_str()) {
            Entry::Occupied(entry) if entry.get() != &parameters => {
                let message = format!(
                    "label was previously used with different parameters `{}({})`",
                    label.as_str(),
                    Parameters(entry.get())
                );
                Err(error(label.as_span(), message))
            }
            Entry::Occupied(entry) => Ok(entry.index()),
            Entry::Vacant(entry) => {
                let i = entry.index();
                entry.insert(parameters);
                Ok(i)
            }
        }?;

        Ok(Self {
            from,
            to,
            role,
            direction,
            edge: GraphEdge::new(label, None),
        })
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

impl<'a> Digraph<'a> {
    fn parse(context: &mut Context<'a>, name: &str, statements: Pair<'a>) -> Result<Self> {
        let (mut nodes, mut edges) = (Vec::new(), Vec::new());
        for statement in statements.into_inner() {
            match statement.as_rule() {
                Rule::node => nodes.push(Node::parse(statement)),
                Rule::edge => edges.push(Edge::parse(context, name, statement)?),
                _ => unreachable!(),
            }
        }

        let mut graph = Graph::with_capacity(nodes.len(), edges.len());
        let nodes = nodes
            .into_iter()
            .map(|node| (node.name, graph.add_node(GraphNode::new(node.name))))
            .collect::<HashMap<_, _>>();

        for edge in edges {
            let (from, to) = (nodes[edge.from], nodes[edge.to]);
            let node = &mut graph[from];
            let (role, direction) = (&mut node.role, &mut node.direction);

            if let Some(role) = role {
                if *role != edge.role.0 {
                    let message =
                        "all outgoing transitions must either send to or receive from the same role";
                    return Err(error(edge.role.1, message.to_owned()));
                }
            }

            if let Some(direction) = direction {
                if *direction != edge.direction.0 {
                    let message = "outgoing transitions must all send or all receive";
                    return Err(error(edge.direction.1, message.to_owned()));
                }
            }

            *role = Some(edge.role.0);
            *direction = Some(edge.direction.0);
            graph.add_edge(from, to, edge.edge);
        }

        Ok(Self { graph })
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
            let mut digraph = Parser::parse(Rule::digraph, input)?;
            let role = next_pair(&mut digraph, Rule::ident).unwrap();
            if !context.roles.insert(role.as_str()) {
                let message = "duplicate graphs found for role";
                return Err(error(role.as_span(), message.to_owned()));
            }

            let statements = next_pair(&mut digraph, Rule::statements).unwrap();
            Ok((role.as_str(), statements, dot_graph))
        });

        let roles = roles
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|(name, statements, dot_graph)| {
                //Ok((name, Digraph::parse(&mut context, name, statements)?.graph))
                //TODO: handle error properly if try_from fails
                Ok((name, Digraph::try_from((dot_graph, &mut context)).unwrap().graph))
            });

        Ok(Tree {
            roles: roles.collect::<Result<Vec<_>>>()?,
            labels: context.labels,
        })
    }
}

fn error(span: Span<'_>, message: String) -> Box<dyn Error> {
    PestError::<Rule>::new_from_span(PestErrorVariant::CustomError { message }, span).into()
}

fn next_pair<'i>(pairs: &mut Pairs<'i>, rule: Rule) -> Option<Pair<'i>> {
    match pairs.next() {
        Some(pair) if pair.as_rule() == rule => Some(pair),
        _ => None,
    }
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
