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
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(B)]
    b: Channel,
    #[route(C)]
    c: Channel,
}

#[derive(Role)]
#[message(Label)]
struct B {
    #[route(A)]
    a: Channel,
    #[route(C)]
    c: Channel,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(A)]
    a: Channel,
    #[route(B)]
    b: Channel,
}

#[derive(Message)]
enum Label {
    Secret(Secret),
    Guess(Guess),
    Less(Less),
    More(More),
    Correct(Correct),
}

struct Secret(i32);

struct Guess(i32);

struct Less(i32);

struct More(i32);

struct Correct(i32);

#[session]
type PlusMinusA = Send<B, Secret, End>;

#[session]
type PlusMinusB = Receive<A, Secret, PlusMinusB1>;

#[session]
type PlusMinusB1 = Receive<C, Guess, Select<C, PlusMinusB3>>;

#[session]
enum PlusMinusB3 {
    Correct(Correct, End),
    More(More, PlusMinusB1),
    Less(Less, PlusMinusB1),
}

#[session]
type PlusMinusC = Send<B, Guess, Branch<B, PlusMinusC2>>;

#[session]
enum PlusMinusC2 {
    Correct(Correct, End),
    More(More, PlusMinusC),
    Less(Less, PlusMinusC),
}
