mod parser;
mod template;

pub use self::template::Protocol;

use self::{
    parser::Tree,
    template::{
        BoolPredicate, Choice, Definition, DefinitionBody, Label, Predicate, Role, Route,
        SideEffect, Type,
    },
};
use heck::{CamelCase, SnakeCase};
use indexmap::IndexMap;
use petgraph::{
    graph::{node_index, NodeIndex},
    visit::{EdgeRef, IntoNodeReferences, VisitMap, Visitable},
};
use std::{error::Error, fs, io, marker::PhantomData, path::Path, result, str};

type Result<T, E = Box<dyn Error>> = result::Result<T, E>;

type Graph<'a> = petgraph::Graph<GraphNode<'a>, GraphEdge<'a>>;
type GraphMap<'a> = <Graph<'a> as Visitable>::Map;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Send,
    Receive,
}

#[derive(Debug)]
struct GraphNode<'a> {
    name: &'a str,
    role: Option<usize>,
    direction: Option<Direction>,
}

impl<'a> GraphNode<'a> {
    fn new(name: &'a str) -> Self {
        Self {
            name,
            role: None,
            direction: None,
        }
    }
}

#[derive(Debug)]
struct GraphEdge<'a> {
    label: usize,
    predicate: BoolPredicate,
    side_effect: SideEffect,
    _marker: PhantomData<&'a usize>,
}

impl<'a> GraphEdge<'a> {
    fn new(label: usize, predicate: BoolPredicate, side_effect: SideEffect) -> Self {
        Self {
            label,
            predicate,
            side_effect,
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, Default)]
pub struct Builder<'a, P: AsRef<Path>> {
    name: &'a str,
    roles: Vec<P>,
}

impl<'a, P: AsRef<Path>> Builder<'a, P> {
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = name;
        self
    }

    pub fn role(mut self, path: P) -> Self {
        self.roles.push(path);
        self
    }

    pub fn generate(self) -> Result<Protocol> {
        if self.name.is_empty() {
            return Err("protocol name was not set in builder".into());
        }

        let inputs = self.roles.iter().map(fs::read_to_string);
        let inputs = inputs.collect::<io::Result<Vec<_>>>()?;
        let tree = Tree::parse(inputs.as_slice())?;

        Ok(Protocol {
            camel: self.name.to_camel_case(),
            roles: generate_roles(&tree.roles),
            labels: generate_labels(&tree.labels),
        })
    }
}

fn generate_nodes(graph: &Graph<'_>) -> Vec<String> {
    graph
        .node_references()
        .map(|(_, node)| node.name.to_camel_case())
        .collect()
}

struct DoublePeekable<I: Iterator> {
    first: Option<I::Item>,
    second: Option<I::Item>,
    remainder: I,
}

impl<I: Iterator> DoublePeekable<I> {
    fn new(mut iterator: I) -> Self {
        Self {
            first: iterator.next(),
            second: iterator.next(),
            remainder: iterator,
        }
    }

    fn is_empty(&self) -> bool {
        self.first.is_none()
    }

    fn singleton(&mut self) -> Option<I::Item> {
        if self.second.is_none() {
            return self.first.take();
        }

        None
    }
}

impl<I: Iterator> Iterator for DoublePeekable<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(first) = self.first.take() {
            return Some(first);
        }

        if let Some(second) = self.second.take() {
            return Some(second);
        }

        self.remainder.next()
    }
}

fn generate_definitions(graph: &Graph<'_>) -> Vec<Definition> {
    struct Visitor<'a> {
        graph: &'a Graph<'a>,
        visited: GraphMap<'a>,
        looped: GraphMap<'a>,
        definitions: Vec<Definition>,
    }

    impl<'a> Visitor<'a> {
        fn new(graph: &'a Graph<'a>) -> Self {
            Self {
                graph,
                visited: graph.visit_map(),
                looped: graph.visit_map(),
                definitions: Vec::new(),
            }
        }

        fn visit(&mut self, node: NodeIndex) -> (Type, bool) {
            let weight = &self.graph[node];
            let mut edges = DoublePeekable::new(self.graph.edges(node));

            if edges.is_empty() {
                assert!(weight.direction.is_none());
                return (Type::End, true);
            }

            if let Some(edge) = edges.singleton() {
                if self.visited.is_visited(&node) {
                    self.looped.visit(node);
                    return (Type::Node(node.index()), false);
                }
                self.visited.visit(node);

                let (next, safe) = self.visit(edge.target());
                let predicate = edge.weight().predicate.clone();
                let side_effect = edge.weight().side_effect.clone();
                let ty = Type::Message {
                    direction: weight.direction.unwrap(),
                    role: weight.role.unwrap(),
                    label: edge.weight().label,
                    predicate: predicate,
                    side_effect: side_effect,
                    next: next.into(),
                };

                if self.looped.is_visited(&node) {
                    self.definitions.push(Definition {
                        node: node.index(),
                        body: DefinitionBody::Type { safe, ty },
                    });
                    return (Type::Node(node.index()), true);
                }

                return (ty, safe);
            }

            let ty = Type::Choice {
                direction: weight.direction.unwrap(),
                role: weight.role.unwrap(),
                node: node.index(),
                predicate: BoolPredicate::Normal(Predicate::Tautology(None)),
                side_effect: SideEffect::None,
            };

            if self.visited.is_visited(&node) {
                self.looped.visit(node);
                return (ty, true);
            }
            self.visited.visit(node);

            let choices = edges
                .map(|edge| Choice {
                    label: edge.weight().label,
                    ty: self.visit(edge.target()).0,
                    predicate: edge.weight().predicate.clone(),
                })
                .collect::<Vec<_>>();
            self.definitions.push(Definition {
                node: node.index(),
                body: DefinitionBody::Choice(choices),
            });

            (ty, true)
        }
    }

    let root = node_index(0);
    let mut visitor = Visitor::new(graph);
    visitor.looped.visit(root);

    let ty = visitor.visit(root).0;
    if ty.is_choice() {
        visitor.definitions.push(Definition {
            node: root.index(),
            body: DefinitionBody::Type { safe: true, ty },
        });
    }

    visitor.definitions
}

fn generate_roles(roles: &[(&str, Graph<'_>)]) -> Vec<Role> {
    roles
        .iter()
        .enumerate()
        .map(|(i, (name, graph))| Role {
            camel: name.to_camel_case(),
            snake: name.to_snake_case(),
            nodes: generate_nodes(graph),
            routes: (0..roles.len()).filter(|&j| j != i).map(Route).collect(),
            definitions: generate_definitions(graph),
        })
        .collect()
}

fn generate_label((label, parameters): (&&str, &Vec<(&str, &str)>)) -> Label {
    let parameters = parameters
        .iter()
        .cloned()
        .map(|(n, t)| (n.to_owned(), t.to_owned()));
    let (names, types): (Vec<String>, Vec<String>) = parameters.unzip();
    Label {
        camel: label.to_camel_case(),
        param_names: names,
        parameters: types,
    }
}

fn generate_labels(labels: &IndexMap<&str, Vec<(&str, &str)>>) -> Vec<Label> {
    labels.iter().map(generate_label).collect()
}
