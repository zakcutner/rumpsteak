use aepl_rumpsteak_derive::from_epl;

use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, Branch, End, Message, Receive, Role, Roles, Select, Send,
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

from_epl!("examples/simple_aepl/A.aepl", SimpleA, A);
from_epl!("examples/simple_aepl/B.aepl", SimpleB, B);

fn main() {
    let Roles{ mut a, mut b } = Roles::default();
    executor::block_on(async {
        try_join!(A(&mut a), B(&mut b)).unwrap();
    });
}
