use super::Direction;
use askama::Template;
use std::fmt::{self, Display, Formatter};

pub(crate) struct Route(pub usize);

pub(crate) enum Type {
    End,
    Definition(usize),
    Message {
        direction: Direction,
        role: usize,
        label: usize,
        next: Box<Self>,
    },
    Choice {
        direction: Direction,
        role: usize,
        index: usize,
    },
}

struct TypeFormatter<'a> {
    ty: &'a Type,
    name: &'a str,
    role: &'a str,
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
}

impl Display for TypeFormatter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.ty {
            Type::End => write!(f, "End"),
            Type::Definition(index) => write!(f, "{}{}{}<'r>", self.name, self.role, index),
            Type::Message {
                direction,
                role,
                label,
                next,
            } => {
                let ty = match direction {
                    Direction::Send => "Send",
                    Direction::Receive => "Receive",
                };

                let (role, other) = (self.role, self.role(role));
                let (label, next) = (self.label(label), self.with(next));
                write!(f, "{}<'r, {}, {}, {}, {}>", ty, role, other, label, next)
            }
            Type::Choice {
                direction,
                role,
                index,
            } => {
                let ty = match direction {
                    Direction::Send => "Select",
                    Direction::Receive => "Branch",
                };

                let (role, other, name) = (self.role, self.role(role), self.name);
                write!(
                    f,
                    "{}<'r, {}, {}, {}{}{}<'r>>",
                    ty, role, other, name, role, index
                )
            }
        }
    }
}

pub(crate) struct Choice {
    pub label: usize,
    pub ty: Type,
}

pub(crate) enum Definition {
    Type { safe: bool, index: usize, ty: Type },
    Choice { index: usize, choices: Vec<Choice> },
}

pub(crate) struct Role {
    pub camel: String,
    pub snake: String,
    pub routes: Vec<Route>,
    pub definitions: Vec<Definition>,
}

pub(crate) struct Label {
    pub camel: String,
    pub parameters: Vec<String>,
}

#[derive(Template)]
#[template(path = "protocol.rs", escape = "none")]
pub struct Protocol {
    pub(crate) camel: String,
    pub(crate) roles: Vec<Role>,
    pub(crate) labels: Vec<Label>,
}

mod filters {
    use super::{Label, Role, Type, TypeFormatter};
    use askama::Result;

    #[allow(clippy::unnecessary_wraps)]
    pub(super) fn copy_bool(t: &bool) -> Result<bool> {
        Ok(*t)
    }

    #[allow(clippy::unnecessary_wraps)]
    pub(super) fn ty<'a>(
        ty: &'a Type,
        name: &'a str,
        role: &'a str,
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
