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

#[derive(Message)]
enum Label {
    Lhs(Lhs),
    Rhs(Rhs),
    Res(Res),
}

struct Lhs(i32);

struct Rhs(i32);

struct Res(i32);

#[session]
type AdderC = Send<S, Lhs, Send<S, Rhs, Receive<S, Res, End>>>;

#[session]
type AdderS = Receive<C, Lhs, Receive<C, Rhs, Send<C, Res, End>>>;
