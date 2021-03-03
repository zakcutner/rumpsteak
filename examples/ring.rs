use futures::{channel::mpsc, executor, try_join};
use rumpsteak::{try_session, End, Message, Receive, Role, Roles, Send};
use std::{error::Error, result};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Sender = mpsc::UnboundedSender<Label>;
type Receiver = mpsc::UnboundedReceiver<Label>;

#[derive(Roles)]
struct Roles(A, B, C);

#[derive(Role)]
#[message(Label)]
struct A(#[route(B)] Sender, #[route(C)] Receiver);

#[derive(Role)]
#[message(Label)]
struct B(#[route(A)] Receiver, #[route(C)] Sender);

#[derive(Role)]
#[message(Label)]
struct C(#[route(A)] Sender, #[route(B)] Receiver);

#[derive(Message)]
enum Label {
    Value(Value),
}

struct Value(i32);

type RingA = Send<B, Value, Receive<C, Value, End>>;

type RingB = Receive<A, Value, Send<C, Value, End>>;

type RingC = Receive<B, Value, Send<A, Value, End>>;

async fn ring_a(role: &mut A, input: i32) -> Result<i32> {
    let x = input;
    try_session(|s: RingA| async {
        let s = s.send(role, Value(x)).await?;
        let (Value(y), s) = s.receive(role).await?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_b(role: &mut B, input: i32) -> Result<i32> {
    let x = input;
    try_session(|s: RingB| async {
        let (Value(y), s) = s.receive(role).await?;
        let s = s.send(role, Value(x)).await?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_c(role: &mut C, input: i32) -> Result<i32> {
    let x = input;
    try_session(|s: RingC| async {
        let (Value(y), s) = s.receive(role).await?;
        let s = s.send(role, Value(x)).await?;
        Ok((x + y, s))
    })
    .await
}

fn main() {
    let Roles(mut a, mut b, mut c) = Roles::default();

    let input = (1, 2, 3);
    println!("input = {:?}", input);

    let output = executor::block_on(async {
        try_join!(
            ring_a(&mut a, input.0),
            ring_b(&mut b, input.1),
            ring_c(&mut c, input.2),
        )
        .unwrap()
    });
    println!("output = {:?}", output);
}
