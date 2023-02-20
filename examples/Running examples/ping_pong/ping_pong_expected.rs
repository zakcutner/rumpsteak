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
    a: A,
    b: B,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(B)]
    b: Channel,
}

#[derive(Role)]
#[message(Label)]
struct B {
    #[route(A)]
    a: Channel,
}

#[derive(Message, Copy, Clone)]
enum Label {
    Ping(Ping),
    Pong(Pong),
}

impl From<Label> for Value {
    fn from(label: Label) -> Value {
        match label {
            Label::Ping(payload) => payload.into(),
            Label::Pong(payload) => payload.into(),
        }
    }
}


#[derive(Copy, Clone)]
struct Ping(i32);

impl From<Ping> for Value {
    fn from(value: Ping) -> Value {
        let Ping(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Pong(i32);

impl From<Pong> for Value {
    fn from(value: Pong) -> Value {
        let Pong(val) = value;
        val
    }
}

#[session(Name, Value)]
struct PingPongA(Send<B, 'x', Ping, Tautology<Name, Value, Ping>, Constant<Name, Value>, Receive<B, 'x', Pong, Tautology<Name, Value, Pong>, Constant<Name, Value>, PingPongA>>);

#[session(Name, Value)]
struct PingPongB(Receive<A, 'x', Ping, Tautology<Name, Value, Ping>, Constant<Name, Value>, Send<A, 'x', Pong, Tautology<Name, Value, Pong>, Constant<Name, Value>, PingPongB>>);
