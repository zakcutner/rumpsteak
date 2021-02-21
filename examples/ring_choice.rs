use futures::{executor, try_join};
use rumpsteak::{
    choice::Choice,
    role::{From, Role, Roles, To},
    try_session, Branch, Label, Receive, Result, Select, Send,
};
use std::convert::Infallible;

#[derive(Roles)]
struct Roles {
    a: A,
    b: B,
    c: C,
}

#[derive(Role)]
#[message(Message)]
struct A {
    #[route(B)]
    b: To<B>,
    #[route(C)]
    c: From<C>,
}

#[derive(Role)]
#[message(Message)]
struct B {
    #[route(A)]
    a: From<A>,
    #[route(C)]
    c: To<C>,
}

#[derive(Role)]
#[message(Message)]
struct C {
    #[route(A)]
    a: To<A>,
    #[route(B)]
    b: From<B>,
}

#[derive(Label)]
enum Message {
    Add(Add),
    Sub(Sub),
}

struct Add(i32);
struct Sub(i32);

type RingA<'r> = Send<'r, A, B, Add, Branch<'r, A, C, RingAChoice<'r>>>;

#[derive(Choice)]
#[role('r, A)]
enum RingAChoice<'r> {
    Add(Add, RingA<'r>),
    Sub(Sub, RingA<'r>),
}

type RingB<'r> = Select<'r, B, C, RingBChoice<'r>>;

#[derive(Choice)]
#[role('r, B)]
enum RingBChoice<'r> {
    Add(Add, Receive<'r, B, A, Add, RingB<'r>>),
    Sub(Sub, Receive<'r, B, A, Add, RingB<'r>>),
}

type RingC<'r> = Branch<'r, C, B, RingCChoice<'r>>;

#[derive(Choice)]
#[role('r, C)]
enum RingCChoice<'r> {
    Add(Add, Send<'r, C, A, Add, RingC<'r>>),
    Sub(Sub, Send<'r, C, A, Sub, RingC<'r>>),
}

async fn ring_a(role: &mut A, mut input: i32) -> Result<Infallible> {
    try_session(role, |mut s: RingA<'_>| async {
        loop {
            println!("A: {}", input);
            let x = input * 2;
            s = match s.send(Add(x))?.branch().await? {
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
    try_session(role, |mut s: RingB<'_>| async {
        loop {
            println!("B: {}", input);
            let x = input * 2;
            s = if x > 0 {
                let s = s.select(Add(x))?;
                let (Add(y), s) = s.receive().await?;
                input = y + x;
                s
            } else {
                let s = s.select(Sub(x))?;
                let (Add(y), s) = s.receive().await?;
                input = y - x;
                s
            };
        }
    })
    .await
}

async fn ring_c(role: &mut C, mut input: i32) -> Result<Infallible> {
    try_session(role, |mut s: RingC<'_>| async {
        loop {
            println!("C: {}", input);
            let x = input * 2;
            s = match s.branch().await? {
                RingCChoice::Add(Add(y), s) => {
                    let s = s.send(Add(x))?;
                    input = x + y;
                    s
                }
                RingCChoice::Sub(Sub(y), s) => {
                    let s = s.send(Sub(x))?;
                    input = x - y;
                    s
                }
            };
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(
            ring_a(&mut roles.a, -1),
            ring_b(&mut roles.b, 0),
            ring_c(&mut roles.c, 1),
        )
        .unwrap();
    });
}
