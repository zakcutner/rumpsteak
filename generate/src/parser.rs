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

struct Edge<'a> {
    from: &'a str,
    to: &'a str,
    role: (usize, Span<'a>),
    direction: (Direction, Span<'a>),
    edge: GraphEdge,
}

impl<'a> Edge<'a> {
    fn parse(context: &mut Context<'a>, name: &str, statement: Pair<'a>) -> Result<Self> {
        let mut pairs = statement.into_inner();

        let from = next_pair(&mut pairs, Rule::ident).unwrap().as_str();
        let to = next_pair(&mut pairs, Rule::ident).unwrap().as_str();

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
            edge: GraphEdge::new(label),
        })
    }
}

struct Digraph<'a> {
    graph: Graph<'a>,
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
            let mut digraph = Parser::parse(Rule::digraph, input)?;
            let role = next_pair(&mut digraph, Rule::ident).unwrap();
            if !context.roles.insert(role.as_str()) {
                let message = "duplicate graphs found for role";
                return Err(error(role.as_span(), message.to_owned()));
            }

            let statements = next_pair(&mut digraph, Rule::statements).unwrap();
            Ok((role.as_str(), statements))
        });

        let roles = roles
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|(name, statements)| {
                Ok((name, Digraph::parse(&mut context, name, statements)?.graph))
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
