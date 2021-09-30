// global protocol ThreeBuyer(role A, role C, role S)
// {
//     empty1(i32) from A to S;
//     empty2(i32) from S to A;
//     empty3(i32) from S to C;
//     empty4(i32) from A to C;
//
//     choice at C
//     {
//         valid(i32) from C to A;
//         valid(i32) from C to S;
//         empty5(i32) from S to C;
//     }
//     or
//     {
//         quit() from C to A;
//         quit() from C to S;
//     }
// }

#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, try_session, Branch, End, Message, Receive, Role, Roles,
    Select, Send,
};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};

use std::error::Error;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
#[allow(dead_code)]
struct Roles {
    c: C,
    s: S,
    a: A,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(S)]
    s: Channel,
    #[route(A)]
    a: Channel,
}

#[derive(Role)]
#[message(Label)]
struct S {
    #[route(C)]
    c: Channel,
    #[route(A)]
    a: Channel,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(C)]
    c: Channel,
    #[route(S)]
    s: Channel,
}

#[derive(Message)]
enum Label {
    Empty3(Empty3),
    Empty4(Empty4),
    Valid(Valid),
    Quit(Quit),
    Empty5(Empty5),
    Empty1(Empty1),
    Empty2(Empty2),
}

struct Empty3(i32);

struct Empty4(i32);

struct Valid(i32);

struct Quit;

struct Empty5(i32);

struct Empty1(i32);

struct Empty2(i32);

#[session]
type ThreeBuyerC = Receive<S, Empty3, (), (), Receive<A, Empty4, (), (), Select<A, (), (), ThreeBuyerC2>>>;

#[session((), ())]
enum ThreeBuyerC2 {
    Quit(Quit, Send<S, Quit, (), (), End<(), ()>>),
    Valid(Valid, Send<S, Valid, (), (), Receive<S, Empty5, (), (), End<(), ()>>>),
}

#[session]
type ThreeBuyerS = Receive<A, Empty1, (), (), Send<A, Empty2, (), (), Send<C, Empty3, (), (), Branch<C, (), (), ThreeBuyerS3>>>>;

#[session((), ())]
enum ThreeBuyerS3 {
    Valid(Valid, Send<C, Empty5, (), (), End<(), ()>>),
    Quit(Quit, End<(), ()>),
}

#[session]
type ThreeBuyerA = Send<S, Empty1, (), (), Receive<S, Empty2, (), (), Send<C, Empty4, (), (), Branch<C, (), (), ThreeBuyerA3>>>>;

#[session((), ())]
enum ThreeBuyerA3 {
    Quit(Quit, End<(), ()>),
    Valid(Valid, End<(), ()>),
}

async fn c(role: &mut C) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: ThreeBuyerC<'_, _>| async {
        let (Empty3(msg_s), s) = s.receive().await?;
        let (Empty4(msg_a), s) = s.receive().await?;
        if msg_a == msg_s {
            // Accept command if both prices are the same
            let s = s.select(Valid(msg_s)).await?;
            let s = s.send(Valid(msg_s)).await?;
            let (Empty5(_msg), s) = s.receive().await?;
            println!("Accept order (price {})", msg_a);
            Ok(((), s))
        } else {
            let s = s.select(Quit).await?;
            let s = s.send(Quit).await?;
            println!("Reject order (price inconsistency {} vs {})", msg_a, msg_s);
            Ok(((), s))
        }
    })
    .await
}

async fn s(role: &mut S) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: ThreeBuyerS<'_, _>| async {
        let (Empty1(msg), s) = s.receive().await?;
        let s = s.send(Empty2(msg)).await?;
        let s = s.send(Empty3(msg)).await?;
        match s.branch().await? {
            ThreeBuyerS3::Valid(Valid(msg), s) => {
                let s = s.send(Empty5(msg)).await?;
                Ok(((), s))
            }
            ThreeBuyerS3::Quit(_, end) => Ok(((), end)),
        }
    })
    .await
}

async fn a(role: &mut A) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: ThreeBuyerA<'_, _>| async {
        let s = s.send(Empty1(42)).await?;
        let (_reply, s) = s.receive().await?;
        let s = s.send(Empty4(42)).await?;
        match s.branch().await? {
            ThreeBuyerA3::Valid(_, end) => Ok(((), end)),
            ThreeBuyerA3::Quit(_, end) => Ok(((), end)),
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(c(&mut roles.c), s(&mut roles.s), a(&mut roles.a)).unwrap();
    });
}
