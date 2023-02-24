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
    SetPw(SetPw),
    Password(Password),
    Success(Success),
    Failure(Failure),
    RetX(RetX),
    RetRes(RetRes),
}

struct SetPw(i32);

struct Password(i32);

struct Success(i32);

struct Failure(i32);

struct RetX(i32);

struct RetRes(i32);

#[session]
type AuthC = Send<S, SetPw, AuthC1>;

#[session]
type AuthC1 = Send<S, Password, Branch<S, AuthC3>>;

#[session]
enum AuthC3 {
    Failure(Failure, Receive<S, RetX, Send<S, RetRes, AuthC1>>),
    Success(Success, End),
}

#[session]
type AuthS = Receive<C, SetPw, AuthS1>;

#[session]
type AuthS1 = Receive<C, Password, Select<C, AuthS3>>;

#[session]
enum AuthS3 {
    Failure(Failure, Send<C, RetX, Receive<C, RetRes, AuthS1>>),
    Success(Success, End),
}
