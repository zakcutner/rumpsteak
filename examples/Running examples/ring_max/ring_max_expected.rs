use ::futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join
};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, Branch, End, Message, Receive, Role, Roles, Select, Send, try_session
};

use std::error::Error;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

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

#[derive(Message)]
enum Label {
    ProposalA(ProposalA),
    ProposalG(ProposalG),
    ProposalB(ProposalB),
    ProposalC(ProposalC),
    ProposalD(ProposalD),
    ProposalE(ProposalE),
    ProposalF(ProposalF),
}

struct ProposalA(i32);

struct ProposalG(i32);

struct ProposalB(i32);

struct ProposalC(i32);

struct ProposalD(i32);

struct ProposalE(i32);

struct ProposalF(i32);

#[session]
type RingMaxA = Send<B, ProposalA, Receive<G, ProposalG, End>>;

#[session]
type RingMaxB = Receive<A, ProposalA, Send<C, ProposalB, End>>;

#[session]
type RingMaxC = Receive<B, ProposalB, Send<D, ProposalC, End>>;

#[session]
type RingMaxD = Receive<C, ProposalC, Send<E, ProposalD, End>>;

#[session]
type RingMaxE = Receive<D, ProposalD, Send<F, ProposalE, End>>;

#[session]
type RingMaxF = Receive<E, ProposalE, Send<G, ProposalF, End>>;

#[session]
type RingMaxG = Receive<F, ProposalF, Send<A, ProposalG, End>>;
