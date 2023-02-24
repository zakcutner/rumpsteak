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
    s: S,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(B)]
    b: Channel,
    #[route(S)]
    s: Channel,
}

#[derive(Role)]
#[message(Label)]
struct B {
    #[route(A)]
    a: Channel,
    #[route(S)]
    s: Channel,
}

#[derive(Role)]
#[message(Label)]
struct S {
    #[route(A)]
    a: Channel,
    #[route(B)]
    b: Channel,
}

#[derive(Message)]
enum Label {
    Request(Request),
    QuoteAlice(QuoteAlice),
    ParticipationBob(ParticipationBob),
    ConfirmAlice(ConfirmAlice),
    QuitAlice(QuitAlice),
    QuoteBob(QuoteBob),
    ConfirmSeller(ConfirmSeller),
    Date(Date),
    QuitSeller(QuitSeller),
}

struct Request(i32);

struct QuoteAlice(i32);

struct ParticipationBob(i32);

struct ConfirmAlice(i32);

struct QuitAlice(i32);

struct QuoteBob(i32);

struct ConfirmSeller(i32);

struct Date(i32);

struct QuitSeller(i32);

#[session]
type ThreeBuyersA = Send<S, Request, Receive<S, QuoteAlice, Send<B, ParticipationBob, Branch<B, ThreeBuyersA3>>>>;

#[session]
enum ThreeBuyersA3 {
    QuitAlice(QuitAlice, End),
    ConfirmAlice(ConfirmAlice, End),
}

#[session]
type ThreeBuyersB = Receive<S, QuoteBob, Receive<A, ParticipationBob, Select<A, ThreeBuyersB2>>>;

#[session]
enum ThreeBuyersB2 {
    QuitAlice(QuitAlice, Send<S, QuitSeller, End>),
    ConfirmAlice(ConfirmAlice, Send<S, ConfirmSeller, Receive<S, Date, End>>),
}

#[session]
type ThreeBuyersS = Receive<A, Request, Send<A, QuoteAlice, Send<B, QuoteBob, Branch<B, ThreeBuyersS3>>>>;

#[session]
enum ThreeBuyersS3 {
    ConfirmSeller(ConfirmSeller, Send<B, Date, End>),
    QuitSeller(QuitSeller, End),
}
