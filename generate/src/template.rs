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

#[derive(Clone, Debug)]
pub(crate) enum Predicate {
    LTnVar(String, String, Option<String>),
    LTnConst(String, String, Option<String>),
    GTnVar(String, String, Option<String>),
    GTnConst(String, String, Option<String>),
    EqualVar(String, String, Option<String>),
    EqualConst(String, String, Option<String>),
    Tautology(Option<String>),
}

impl Display for Predicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Predicate::LTnVar(param, value, label) => {
		match label {
			None => write!(f, "LTnVar::<Value, Label, '{}', '{}'>", param, value),
			Some(l) => write!(f, "LTnVar::<Value, {}, '{}', '{}'>", l, param, value),
		}
            }
            Predicate::LTnConst(param, value, label) => {
		match label {
			None => write!(f, "LTnConst::<Label, '{}', {}>", param, value),
			Some(l) => write!(f, "LTnConst::<{}, '{}', {}>", l, param, value),
		}
            }
            Predicate::GTnVar(param, value, label) => {
		match label {
			None => write!(f, "GTnVar::<Value, Label, '{}', '{}'>", param, value),
			Some(l) => write!(f, "GTnVar::<Value, {}, '{}', '{}'>", l, param, value),
		}
            }
            Predicate::GTnConst(param, value, label) => {
		match label {
			None => write!(f, "GTnConst::<Label, '{}', {}>", param, value),
			Some(l) => write!(f, "GTnConst::<{}, '{}', {}>", l, param, value),
		}
            }
            Predicate::EqualVar(param, value, label) => {
		match label {
			None => write!(f, "EqualVar::<Value, Label, '{}', '{}'>", param, value),
			Some(l) => write!(f, "EqualVar::<Value, {}, '{}', '{}'>", l, param, value),
		}
            }
            Predicate::EqualConst(param, value, label) => {
		match label {
			None => write!(f, "EqualConst::<Label, '{}', {}>", param, value),
			Some(l) => write!(f, "EqualConst::<{}, '{}', {}>", l, param, value),
		}
            }
            Predicate::Tautology(label) => {
                match label {
                    None => write!(f, "Tautology::<Name, Value, Label>"),
                    Some(l) => write!(f, "Tautology::<Name, Value, {}>", l),
                }
            }
        }
    }
}

impl Predicate {
    fn set_label_str(&mut self, label: String) {
        match self {
	    Predicate::LTnVar(_, _, opt) |
	    Predicate::LTnConst(_, _, opt) |
	    Predicate::GTnVar(_, _, opt) |
	    Predicate::GTnConst(_, _, opt) |
	    Predicate::EqualVar(_, _, opt) |
	    Predicate::EqualConst(_, _, opt) |
            Predicate::Tautology(opt) => {
                opt.insert(label);
            },
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate) enum BoolPredicate {
    Normal(Predicate),
    And(Option<String>, Box<BoolPredicate>, Box<BoolPredicate>),
    Or(Option<String>, Box<BoolPredicate>, Box<BoolPredicate>),
    Neg(Option<String>, Box<BoolPredicate>),
}

impl Display for BoolPredicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BoolPredicate::Normal(a) => {
                write!(f, "{}", a)
            }
            BoolPredicate::Neg(l, a) => {
                match l {
                Some(l) => write!(f, "Neg::<{}, {}, Name, Value>", l, a),
                None => write!(f, "Neg::<Label, {}, Name, Value>", a),
                }
            }
            BoolPredicate::And(_l, a, b) => {
                write!(f, "And<{}, {}>", a, b)
            }
            BoolPredicate::Or(l, a, b) => {
                match l {
                    Some(l) => 
                        write!(f, "Or<{}, {}, {}, Name, Value>", l, a, b),
                    None =>
                        write!(f, "Or<Label, {}, {}, Name, Value>", a, b),
                }
            }
        }
    }
}

impl BoolPredicate {
    fn set_label_str(&mut self, label: String) {
        match self {
            BoolPredicate::Normal(p) => p.set_label_str(label),
            BoolPredicate::Neg(l, p) => {
                l.insert(label.clone());
                p.set_label_str(label)
            },
            BoolPredicate::And(l, p1, p2) |
                BoolPredicate::Or(l, p1, p2) => {
                    l.insert(label.clone());
                    p1.set_label_str(label.clone());
                    p2.set_label_str(label)
                }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum SideEffect {
    Increase(String, String),
    Decrease(String, String),
    Multiply(String, String),
    Divide(String, String),
    None,
}

#[derive(Debug)]
pub(crate) enum Type {
    End,
    Node(usize),
    Message {
        direction: Direction,
        role: usize,
        label: usize,
        predicate: BoolPredicate,
        side_effect: SideEffect,
        next: Box<Self>,
    },
    Choice {
        direction: Direction,
        role: usize,
        node: usize,
        predicate: BoolPredicate,
        side_effect: SideEffect,
    },
}

impl Type {
    pub(crate) fn is_choice(&self) -> bool {
        matches!(
            self,
            Self::Choice {
                direction: _,
                role: _,
                node: _,
                predicate: _,
                side_effect: _,
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

    fn param_names(&self, label: &usize) -> Vec<&str> {
        self.labels[*label].param_names
            .iter()
            .map(|s| s as &str)
            .collect()
    }

    fn node(&self, node: &usize) -> &str {
        &self.role.nodes[*node]
    }

    fn boolpred(&self, predicate: &BoolPredicate) -> String {
        predicate.to_string()
    }

    fn effect(&self, side_effect: &SideEffect) -> String {
        match side_effect {
            SideEffect::Increase(param, value) => {
                let mut effect = String::from("Incr<'");
                effect = effect + param;
                effect = effect + "', ";
                effect = effect + value;
                effect = effect + ">";
                return effect;
            }
            SideEffect::Decrease(param, value) => {
                let mut effect = String::from("Decr<'");
                effect = effect + param;
                effect = effect + "', ";
                effect = effect + value;
                effect = effect + ">";
                return effect;
            }
            SideEffect::Multiply(param, value) => {
                let mut effect = String::from("Mult<'");
                effect = effect + param;
                effect = effect + "', ";
                effect = effect + value;
                effect = effect + ">";
                return effect;
            }
            SideEffect::Divide(param, value) => {
                let mut effect = String::from("Div<'");
                effect = effect + param;
                effect = effect + "', ";
                effect = effect + value;
                effect = effect + ">";
                return effect;
            }
            SideEffect::None => (),
        }
        return "Constant<Name, Value>".to_string();
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
                predicate,
                side_effect,
                next,
            } => {
                let (other, param_name, label, effect, next) = (
                    self.role(role),
                    self.param_names(label).iter().next().unwrap().clone(),
                    self.label(label),
                    self.effect(side_effect),
                    self.with(next),
                );
                let mut predicate = predicate.clone();
                predicate.set_label_str(label.to_string());
                let pred = self.boolpred(&mut predicate);
                match direction {
                    Direction::Send => write!(
                        f,
                        "Send<{}, '{}', {}, {}, {}, {}>",
                        other, param_name, label, pred, effect, next
                    ),
                    Direction::Receive => write!(
                        f,
                        "Receive<{}, '{}', {}, {}, {}, {}>",
                        other, param_name, label, pred, effect, next
                    ),
                }
            }
            Type::Choice {
                direction,
                role,
                node,
                predicate,
                side_effect,
            } => {
                let (other, name, role, node, pred, effect) = (
                    self.role(role),
                    self.name,
                    &self.role.camel,
                    self.node(node),
                    self.boolpred(predicate),
                    self.effect(side_effect),
                );
                match direction {
                    Direction::Send => {
                        write!(
                            f,
                            "Select<{}, {}{}{}Predicate, {}, {}{}{}>",
                            other, name, role, node, effect, name, role, node
                        )
                    }
                    Direction::Receive => {
                        write!(
                            f,
                            "Branch<{}, {}, {}, {}{}{}>",
                            other, pred, effect, name, role, node
                        )
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
    pub predicate: BoolPredicate,
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
