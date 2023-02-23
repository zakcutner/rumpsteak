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
    c: C,
    d: D,
    e: E,
    f: F,
    g: G,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(B)]
    b: Channel,
    #[route(C)]
    c: Channel,
    #[route(D)]
    d: Channel,
    #[route(E)]
    e: Channel,
    #[route(F)]
    f: Channel,
    #[route(G)]
    g: Channel,
}

#[derive(Role)]
#[message(Label)]
struct B {
    #[route(A)]
    a: Channel,
    #[route(C)]
    c: Channel,
    #[route(D)]
    d: Channel,
    #[route(E)]
    e: Channel,
    #[route(F)]
    f: Channel,
    #[route(G)]
    g: Channel,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(A)]
    a: Channel,
    #[route(B)]
    b: Channel,
    #[route(D)]
    d: Channel,
    #[route(E)]
    e: Channel,
    #[route(F)]
    f: Channel,
    #[route(G)]
    g: Channel,
}

#[derive(Role)]
#[message(Label)]
struct D {
    #[route(A)]
    a: Channel,
    #[route(B)]
    b: Channel,
    #[route(C)]
    c: Channel,
    #[route(E)]
    e: Channel,
    #[route(F)]
    f: Channel,
    #[route(G)]
    g: Channel,
}

#[derive(Role)]
#[message(Label)]
struct E {
    #[route(A)]
    a: Channel,
    #[route(B)]
    b: Channel,
    #[route(C)]
    c: Channel,
    #[route(D)]
    d: Channel,
    #[route(F)]
    f: Channel,
    #[route(G)]
    g: Channel,
}

#[derive(Role)]
#[message(Label)]
struct F {
    #[route(A)]
    a: Channel,
    #[route(B)]
    b: Channel,
    #[route(C)]
    c: Channel,
    #[route(D)]
    d: Channel,
    #[route(E)]
    e: Channel,
    #[route(G)]
    g: Channel,
}

#[derive(Role)]
#[message(Label)]
struct G {
    #[route(A)]
    a: Channel,
    #[route(B)]
    b: Channel,
    #[route(C)]
    c: Channel,
    #[route(D)]
    d: Channel,
    #[route(E)]
    e: Channel,
    #[route(F)]
    f: Channel,
}

#[derive(Message, Copy, Clone)]
enum Label {
    ProposalA(ProposalA),
    ProposalG(ProposalG),
    ProposalB(ProposalB),
    ProposalC(ProposalC),
    ProposalD(ProposalD),
    ProposalE(ProposalE),
    ProposalF(ProposalF),
}

impl From<Label> for Value {
    fn from(label: Label) -> Value {
        match label {
            Label::ProposalA(payload) => payload.into(),
            Label::ProposalG(payload) => payload.into(),
            Label::ProposalB(payload) => payload.into(),
            Label::ProposalC(payload) => payload.into(),
            Label::ProposalD(payload) => payload.into(),
            Label::ProposalE(payload) => payload.into(),
            Label::ProposalF(payload) => payload.into(),
        }
    }
}


#[derive(Copy, Clone)]
struct ProposalA(i32);

impl From<ProposalA> for Value {
    fn from(value: ProposalA) -> Value {
        let ProposalA(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct ProposalG(i32);

impl From<ProposalG> for Value {
    fn from(value: ProposalG) -> Value {
        let ProposalG(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct ProposalB(i32);

impl From<ProposalB> for Value {
    fn from(value: ProposalB) -> Value {
        let ProposalB(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct ProposalC(i32);

impl From<ProposalC> for Value {
    fn from(value: ProposalC) -> Value {
        let ProposalC(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct ProposalD(i32);

impl From<ProposalD> for Value {
    fn from(value: ProposalD) -> Value {
        let ProposalD(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct ProposalE(i32);

impl From<ProposalE> for Value {
    fn from(value: ProposalE) -> Value {
        let ProposalE(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct ProposalF(i32);

impl From<ProposalF> for Value {
    fn from(value: ProposalF) -> Value {
        let ProposalF(val) = value;
        val
    }
}

#[session(Name, Value)]
type RingMaxA = Send<B, 'a', ProposalA, Tautology::<Name, Value, ProposalA>, Constant<Name, Value>, Receive<G, 'g', ProposalG, Or<ProposalG, EqualVar::<Value, Label, 'g', 'f'>, GTnVar::<Value, Label, 'g', 'f'>, Name, Value>, Constant<Name, Value>, End>>;

#[session(Name, Value)]
type RingMaxB = Receive<A, 'a', ProposalA, Tautology::<Name, Value, ProposalA>, Constant<Name, Value>, Send<C, 'b', ProposalB, Or<ProposalB, EqualVar::<Value, Label, 'b', 'a'>, GTnVar::<Value, Label, 'b', 'a'>, Name, Value>, Constant<Name, Value>, End>>;

#[session(Name, Value)]
type RingMaxC = Receive<B, 'b', ProposalB, Or<ProposalB, EqualVar::<Value, Label, 'b', 'a'>, GTnVar::<Value, Label, 'b', 'a'>, Name, Value>, Constant<Name, Value>, Send<D, 'c', ProposalC, Or<ProposalC, EqualVar::<Value, Label, 'c', 'b'>, GTnVar::<Value, Label, 'c', 'b'>, Name, Value>, Constant<Name, Value>, End>>;

#[session(Name, Value)]
type RingMaxD = Receive<C, 'c', ProposalC, Or<ProposalC, EqualVar::<Value, Label, 'c', 'b'>, GTnVar::<Value, Label, 'c', 'b'>, Name, Value>, Constant<Name, Value>, Send<E, 'd', ProposalD, Or<ProposalD, EqualVar::<Value, Label, 'd', 'c'>, GTnVar::<Value, Label, 'd', 'c'>, Name, Value>, Constant<Name, Value>, End>>;

#[session(Name, Value)]
type RingMaxE = Receive<D, 'd', ProposalD, Or<ProposalD, EqualVar::<Value, Label, 'd', 'c'>, GTnVar::<Value, Label, 'd', 'c'>, Name, Value>, Constant<Name, Value>, Send<F, 'e', ProposalE, Or<ProposalE, EqualVar::<Value, Label, 'e', 'd'>, GTnVar::<Value, Label, 'e', 'd'>, Name, Value>, Constant<Name, Value>, End>>;

#[session(Name, Value)]
type RingMaxF = Receive<E, 'e', ProposalE, Or<ProposalE, EqualVar::<Value, Label, 'e', 'd'>, GTnVar::<Value, Label, 'e', 'd'>, Name, Value>, Constant<Name, Value>, Send<G, 'f', ProposalF, Or<ProposalF, EqualVar::<Value, Label, 'f', 'e'>, GTnVar::<Value, Label, 'f', 'e'>, Name, Value>, Constant<Name, Value>, End>>;

#[session(Name, Value)]
type RingMaxG = Receive<F, 'f', ProposalF, Or<ProposalF, EqualVar::<Value, Label, 'f', 'e'>, GTnVar::<Value, Label, 'f', 'e'>, Name, Value>, Constant<Name, Value>, Send<A, 'g', ProposalG, Or<ProposalG, EqualVar::<Value, Label, 'g', 'f'>, GTnVar::<Value, Label, 'g', 'f'>, Name, Value>, Constant<Name, Value>, End>>;
