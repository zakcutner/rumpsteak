use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
use rumpsteak::{channel::Bidirectional, try_session, End, Message, Receive, Role, Roles, Send};
use std::{error::Error, result};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(A, B, C);

#[derive(Role)]
#[message(Label)]
struct A(#[route(B)] Channel, #[route(C)] Channel);

#[derive(Role)]
#[message(Label)]
struct B(#[route(A)] Channel, #[route(C)] Channel);

#[derive(Role)]
#[message(Label)]
struct C(#[route(A)] Channel, #[route(B)] Channel);

#[derive(Message)]
enum Label {
    Add(Add),
    Sum(Sum),
}

struct Add(i32);
struct Sum(i32);

type AdderA = Send<B, Add, Receive<B, Add, Send<C, Add, Receive<C, Sum, End>>>>;

type AdderB = Receive<A, Add, Send<A, Add, Send<C, Add, Receive<C, Sum, End>>>>;

type AdderC = Receive<A, Add, Receive<B, Add, Send<A, Sum, Send<B, Sum, End>>>>;

async fn adder_a(role: &mut A) -> Result<()> {
    try_session(|s: AdderA| async {
        let x = 2;
        let s = s.send(role, Add(x)).await?;
        let (Add(y), s) = s.receive(role).await?;
        let s = s.send(role, Add(y)).await?;
        let (Sum(z), s) = s.receive(role).await?;
        println!("{} + {} = {}", x, y, z);
        assert_eq!(z, 5);
        Ok(((), s))
    })
    .await
}

async fn adder_b(role: &mut B) -> Result<()> {
    try_session(|s: AdderB| async {
        let (Add(y), s) = s.receive(role).await?;
        let x = 3;
        let s = s.send(role, Add(x)).await?;
        let s = s.send(role, Add(y)).await?;
        let (Sum(z), s) = s.receive(role).await?;
        println!("{} + {} = {}", x, y, z);
        assert_eq!(z, 5);
        Ok(((), s))
    })
    .await
}

async fn adder_c(role: &mut C) -> Result<()> {
    try_session(|s: AdderC| async {
        let (Add(x), s) = s.receive(role).await?;
        let (Add(y), s) = s.receive(role).await?;
        let z = x + y;
        let s = s.send(role, Sum(z)).await?;
        Ok(((), s.send(role, Sum(z)).await?))
    })
    .await
}

fn main() {
    let Roles(mut a, mut b, mut c) = Roles::default();
    executor::block_on(async {
        try_join!(adder_a(&mut a), adder_b(&mut b), adder_c(&mut c)).unwrap();
    });
}
