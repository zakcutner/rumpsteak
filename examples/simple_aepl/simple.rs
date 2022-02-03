use aepl_rumpsteak_derive::from_epl;

use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, Branch, End, Message, Receive, Role, Roles, Select, Send, try_session
};

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

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

#[derive(Message)]
enum Label {
    Ping(Ping),
}

struct Ping(u32);

#[session]
type SimpleA = Send<B, Ping, End>;

#[session]
type SimpleB = Receive<A, Ping, End>;

fn new_msg() -> Ping {
    Ping(1)
}

fn print_msg(p: Ping) {
    let Ping(n) = p;
    println!("Received {}", n);
}

from_epl!("examples/simple_aepl/A.aepl", SimpleA, A);
from_epl!("examples/simple_aepl/B.aepl", SimpleB, B);

async fn a(role: &mut A) -> Result<(), Box<dyn std::error::Error>> {
    try_session(role, |a: SimpleA<'_, _>| async {
        A(a).await
    }).await
}

async fn b(role: &mut B) -> Result<(), Box<dyn std::error::Error>> {
    try_session(role, |b: SimpleB<'_, _>| async {
        B(b).await
    }).await
}

fn main() {
    let Roles{ a: mut role_a, b: mut role_b } = Roles::default();
    executor::block_on(async {
        try_join!(a(&mut role_a), b(&mut role_b)).unwrap();
    });
}
