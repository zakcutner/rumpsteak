use super::Direction;
use askama::Template;
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, Write},
    path::Path,
};

#[derive(Debug)]
pub(crate) struct Route(pub usize);

#[derive(Debug)]
pub(crate) enum Type {
    End,
    Node(usize),
    Message {
        direction: Direction,
        role: usize,
        label: usize,
        next: Box<Self>,
    },
    Choice {
        direction: Direction,
        role: usize,
        node: usize,
    },
}

impl Type {
    pub(crate) fn is_choice(&self) -> bool {
        matches!(
            self,
            Self::Choice {
                direction: _,
                role: _,
                node: _
            }
        )
    }
}

struct TypeFormatter<'a> {
    ty: &'a Type,
    name: &'a str,
    role: &'a Role,
    roles: &'a [Role],
    labels: &'a [Label],
}

impl<'a> TypeFormatter<'a> {
    fn with(&self, ty: &'a Type) -> Self {
        Self {
            ty,
            name: self.name,
            role: self.role,
            roles: self.roles,
            labels: self.labels,
        }
    }

    fn role(&self, role: &usize) -> &str {
        &self.roles[*role].camel
    }

    fn label(&self, label: &usize) -> &str {
        &self.labels[*label].camel
    }

    fn node(&self, node: &usize) -> &str {
        &self.role.nodes[*node]
    }
}

impl Display for TypeFormatter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.ty {
            Type::End => write!(f, "End"),
            Type::Node(node) if *node > 0 => {
                write!(f, "{}{}{}", self.name, self.role.camel, self.node(node))
            }
            Type::Node(_) => write!(f, "{}{}", self.name, self.role.camel),
            Type::Message {
                direction,
                role,
                label,
                next,
            } => {
                let (other, label, next) = (self.role(role), self.label(label), self.with(next));
                match direction {
                    Direction::Send => write!(f, "Send<{}, {}, {}>", other, label, next),
                    Direction::Receive => write!(f, "Receive<{}, {}, {}>", other, label, next),
                }
            }
            Type::Choice {
                direction,
                role,
                node,
            } => {
                let other = self.role(role);
                let (name, role, node) = (self.name, &self.role.camel, self.node(node));
                match direction {
                    Direction::Send => {
                        write!(f, "Select<{}, {}{}{}>", other, name, role, node)
                    }
                    Direction::Receive => {
                        write!(f, "Branch<{}, {}{}{}>", other, name, role, node)
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Choice {
    pub label: usize,
    pub ty: Type,
}

#[derive(Debug)]
pub(crate) enum DefinitionBody {
    Type { safe: bool, ty: Type },
    Choice(Vec<Choice>),
}

#[derive(Debug)]
pub(crate) struct Definition {
    pub node: usize,
    pub body: DefinitionBody,
}

#[derive(Debug)]
pub(crate) struct Role {
    pub camel: String,
    pub snake: String,
    pub nodes: Vec<String>,
    pub routes: Vec<Route>,
    pub definitions: Vec<Definition>,
}

#[derive(Debug)]
pub(crate) struct Label {
    pub camel: String,
    pub param_names: Vec<String>,
    pub parameters: Vec<String>,
}

#[derive(Debug, Template)]
#[template(path = "protocol.rs", escape = "none")]
pub struct Protocol {
    pub(crate) camel: String,
    pub(crate) roles: Vec<Role>,
    pub(crate) labels: Vec<Label>,
    pub(crate) name_str: String,
    pub(crate) value_str: String,
}

impl Protocol {
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        write!(File::create(path)?, "{}", self)
    }
}

mod filters {
    use super::{Label, Role, Type, TypeFormatter};
    use askama::Result;

    #[allow(clippy::unnecessary_wraps)]
    pub(super) fn copy_bool(b: &bool) -> Result<bool> {
        Ok(*b)
    }

    #[allow(clippy::unnecessary_wraps)]
    pub(super) fn ty<'a>(
        ty: &'a Type,
        name: &'a str,
        role: &'a Role,
        roles: &'a [Role],
        labels: &'a [Label],
    ) -> Result<TypeFormatter<'a>> {
        Ok(TypeFormatter {
            ty,
            name,
            role,
            roles,
            labels,
        })
    }
}
