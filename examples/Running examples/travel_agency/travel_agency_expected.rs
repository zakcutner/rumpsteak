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
    a: A,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(A)]
    a: Channel,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(C)]
    c: Channel,
}

#[derive(Message)]
enum Label {
    Order(Order),
    Quote(Quote),
    Accept(Accept),
    Reject(Reject),
    Address(Address),
    Date(Date),
}

struct Order(i32);

struct Quote(i32);

struct Accept(i32);

struct Reject(i32);

struct Address(i32);

struct Date(i32);

#[session]
type TravelAgencyC = Send<A, Order, Receive<A, Quote, Select<A, TravelAgencyC2>>>;

#[session]
enum TravelAgencyC2 {
    Reject(Reject, End),
    Accept(Accept, Send<A, Address, Receive<A, Date, End>>),
}

#[session]
type TravelAgencyA = Receive<C, Order, Send<C, Quote, Branch<C, TravelAgencyA2>>>;

#[session]
enum TravelAgencyA2 {
    Reject(Reject, End),
    Accept(Accept, Receive<C, Address, Send<C, Date, End>>),
}
