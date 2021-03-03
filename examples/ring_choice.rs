use futures::{channel::mpsc, executor, try_join};
use rumpsteak::{try_session, Branch, Choice, Message, Receive, Role, Roles, Select, Send};
use std::{convert::Infallible, error::Error, result};

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
    Add(Add),
    Sub(Sub),
}

struct Add(i32);
struct Sub(i32);

type RingA = Send<B, Add, Branch<C, RingAChoice>>;

#[derive(Choice)]
#[message(Label)]
enum RingAChoice {
    Add(Add, RingA),
    Sub(Sub, RingA),
}

type RingB = Select<C, RingBChoice>;

#[derive(Choice)]
#[message(Label)]
enum RingBChoice {
    Add(Add, Receive<A, Add, RingB>),
    Sub(Sub, Receive<A, Add, RingB>),
}

type RingC = Branch<B, RingCChoice>;

#[derive(Choice)]
#[message(Label)]
enum RingCChoice {
    Add(Add, Send<A, Add, RingC>),
    Sub(Sub, Send<A, Sub, RingC>),
}

async fn ring_a(role: &mut A, mut input: i32) -> Result<Infallible> {
    try_session(|mut s: RingA| async {
        loop {
            println!("A: {}", input);
            let x = input * 2;
            s = match s.send(role, Add(x)).await?.branch(role).await? {
                RingAChoice::Add(Add(y), s) => {
                    input = x + y;
                    s
                }
                RingAChoice::Sub(Sub(y), s) => {
                    input = x - y;
                    s
                }
            };
        }
    })
    .await
}

async fn ring_b(role: &mut B, mut input: i32) -> Result<Infallible> {
    try_session(|mut s: RingB| async {
        loop {
            println!("B: {}", input);
            let x = input * 2;
            s = if x > 0 {
                let s = s.select(role, Add(x)).await?;
                let (Add(y), s) = s.receive(role).await?;
                input = y + x;
                s
            } else {
                let s = s.select(role, Sub(x)).await?;
                let (Add(y), s) = s.receive(role).await?;
                input = y - x;
                s
            };
        }
    })
    .await
}

async fn ring_c(role: &mut C, mut input: i32) -> Result<Infallible> {
    try_session(|mut s: RingC| async {
        loop {
            println!("C: {}", input);
            let x = input * 2;
            s = match s.branch(role).await? {
                RingCChoice::Add(Add(y), s) => {
                    let s = s.send(role, Add(x)).await?;
                    input = x + y;
                    s
                }
                RingCChoice::Sub(Sub(y), s) => {
                    let s = s.send(role, Sub(x)).await?;
                    input = x - y;
                    s
                }
            };
        }
    })
    .await
}

fn main() {
    let Roles(mut a, mut b, mut c) = Roles::default();
    executor::block_on(async {
        try_join!(ring_a(&mut a, -1), ring_b(&mut b, 0), ring_c(&mut c, 1)).unwrap();
    });
}
