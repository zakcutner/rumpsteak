use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional,
    session,
    Branch,
    End,
    Message,
    Receive,
    Role,
    Roles,
    Select,
    Send,
    effect::{
        SideEffect,
        Constant,
        Incr,
    },
    try_session,
    predicate::*,
    ParamName,
    Param,
};

use std::collections::HashMap;
use std::error::Error;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

type Name = char;
type Value = i32;

#[derive(Roles)]
#[allow(dead_code)]
struct Roles {
    c: C,
    s: S,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(S)]
    s: Channel,
}

#[derive(Role)]
#[message(Label)]
struct S {
    #[route(C)]
    c: Channel,
}

#[derive(Message, Copy, Clone)]
enum Label {
    Lhs(Lhs),
    Quit(Quit),
    Rhs(Rhs),
    Ok(Ok),
    Ret(Ret),
}

impl From<Label> for Value {
    fn from(label: Label) -> Value {
        match label {
            Label::Lhs(payload) => payload.into(),
            Label::Quit(payload) => payload.into(),
            Label::Rhs(payload) => payload.into(),
            Label::Ok(payload) => payload.into(),
            Label::Ret(payload) => payload.into(),
        }
    }
}


#[derive(Copy, Clone)]
struct Lhs(i32);

impl From<Lhs> for Value {
    fn from(value: Lhs) -> Value {
        let Lhs(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Quit(i32);

impl From<Quit> for Value {
    fn from(value: Quit) -> Value {
        let Quit(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Rhs(i32);

impl From<Rhs> for Value {
    fn from(value: Rhs) -> Value {
        let Rhs(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Ok(i32);

impl From<Ok> for Value {
    fn from(value: Ok) -> Value {
        let Ok(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Ret(i32);

impl From<Ret> for Value {
    fn from(value: Ret) -> Value {
        let Ret(val) = value;
        val
    }
}

#[session(Name, Value)]
type AdderC = Select<S, AdderC0Predicate, Constant<Name, Value>, AdderC0>;

#[session(Name, Value)]
enum AdderC0 {
    Quit(Quit, End),
    Lhs(Lhs, Send<S, 'y', Rhs, Tautology::<Name, Value, Rhs>, Constant<Name, Value>, Receive<S, 'x', Ok, Tautology::<Name, Value, Ok>, Constant<Name, Value>, Receive<S, 'y', Ret, Tautology::<Name, Value, Ret>, Constant<Name, Value>, Select<S, AdderC0Predicate, Constant<Name, Value>, AdderC0>>>>),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for AdderC0<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::Quit(Quit(val)) => {
                    ('z', *val)
            }
            Label::Lhs(Lhs(val)) => {
                    ('x', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Quit, Name> for AdderC0<'__r, __R> {
    fn get_param_name() -> Name {
        'z'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Lhs, Name> for AdderC0<'__r, __R> {
    fn get_param_name() -> Name {
        'x'
    }
}

#[derive(Default)]
struct AdderC0Predicate {}
impl Predicate for AdderC0Predicate {
    type Name = Name;
    type Value = Value;
    type Label = Label;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        label: Option<&Self::Label>
    ) -> Result<(), Self::Error> {
        if let Some(label) = label {
            match label {
                Label::Quit(_) => {
                    Tautology::<Name, Value, Label>::default()
                        .check(m, Some(label))
                    },
                Label::Lhs(_) => {
                    Tautology::<Name, Value, Label>::default()
                        .check(m, Some(label))
                    },
                _ => {
                    Err(())
                }
            }
        } else {
            Err(())
        }
    }
}

#[session(Name, Value)]
type AdderS = Branch<C, Tautology::<Name, Value, Label>, Constant<Name, Value>, AdderS0>;

#[session(Name, Value)]
enum AdderS0 {
    Quit(Quit, End),
    Lhs(Lhs, Receive<C, 'y', Rhs, Tautology::<Name, Value, Rhs>, Constant<Name, Value>, Send<C, 'x', Ok, Tautology::<Name, Value, Ok>, Constant<Name, Value>, Send<C, 'y', Ret, Tautology::<Name, Value, Ret>, Constant<Name, Value>, Branch<C, Tautology::<Name, Value, Label>, Constant<Name, Value>, AdderS0>>>>),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for AdderS0<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::Quit(Quit(val)) => {
                    ('z', *val)
            }
            Label::Lhs(Lhs(val)) => {
                    ('x', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Quit, Name> for AdderS0<'__r, __R> {
    fn get_param_name() -> Name {
        'z'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Lhs, Name> for AdderS0<'__r, __R> {
    fn get_param_name() -> Name {
        'x'
    }
}

#[derive(Default)]
struct AdderS0Predicate {}
impl Predicate for AdderS0Predicate {
    type Name = Name;
    type Value = Value;
    type Label = Label;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        label: Option<&Self::Label>
    ) -> Result<(), Self::Error> {
        if let Some(label) = label {
            match label {
                Label::Quit(_) => {
                    Tautology::<Name, Value, Label>::default()
                        .check(m, Some(label))
                    },
                Label::Lhs(_) => {
                    Tautology::<Name, Value, Label>::default()
                        .check(m, Some(label))
                    },
                _ => {
                    Err(())
                }
            }
        } else {
            Err(())
        }
    }
}
