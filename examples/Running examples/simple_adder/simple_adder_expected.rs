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
    Rhs(Rhs),
    Res(Res),
}

impl From<Label> for Value {
    fn from(label: Label) -> Value {
        match label {
            Label::Lhs(payload) => payload.into(),
            Label::Rhs(payload) => payload.into(),
            Label::Res(payload) => payload.into(),
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
struct Rhs(i32);

impl From<Rhs> for Value {
    fn from(value: Rhs) -> Value {
        let Rhs(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Res(i32);

impl From<Res> for Value {
    fn from(value: Res) -> Value {
        let Res(val) = value;
        val
    }
}

#[session(Name, Value)]
type AdderC = Send<S, 'x', Lhs, Tautology<Name, Value, Lhs>, Constant<Name, Value>, Send<S, 'y', Rhs, Tautology<Name, Value, Rhs>, Constant<Name, Value>, Receive<S, 'r', Res, Tautology<Name, Value, Res>, Constant<Name, Value>, End>>>;

#[session(Name, Value)]
type AdderS = Receive<C, 'x', Lhs, Tautology<Name, Value, Lhs>, Constant<Name, Value>, Receive<C, 'y', Rhs, Tautology<Name, Value, Rhs>, Constant<Name, Value>, Send<C, 'r', Res, Tautology<Name, Value, Res>, Constant<Name, Value>, End>>>;
